---
component: ColorPicker
category: specialized
tier: complex
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: [angle-slider, color-area, color-field, color-slider, color-swatch, color-swatch-picker, color-wheel]
references:
    ark-ui: ColorPicker
    react-aria: ColorPicker
---

# ColorPicker

A `ColorPicker` lets the user select a color through a combination of a 2D area (saturation
and lightness), a hue channel strip, an optional alpha channel strip, and optional text
inputs for individual channels. It supports multiple color formats (HSL, HSB, RGB, HEX)
and includes an optional EyeDropper browser API integration.

## Color Contrast Responsibility

`ColorPicker` is a **data entry component**; contrast validation of the selected color is the **application's responsibility**, not the component's. The component provides a color selection UI — it does not (and cannot) know how the selected color will be used (background, text, border, etc.).

- Adapters MAY provide an optional **contrast checker widget** (e.g., a badge showing the WCAG contrast ratio against a reference color) as a composable companion component, but this is NOT part of the core `ColorPicker` machine.
- Applications that use `ColorPicker` for theme building or text color selection SHOULD validate the selected color against WCAG 2.1 contrast requirements: **4.5:1** minimum for normal text, **3:1** for large text (>=18pt or >=14pt bold).
- The `ColorValue` type exposes a `relative_luminance() -> f64` method that can be used for contrast ratio calculation: `contrast_ratio = (L1 + 0.05) / (L2 + 0.05)` where `L1` is the lighter luminance.

## Types

The internal color representation is `ColorValue` (HSL + alpha). All other spaces are
computed on demand via conversion methods on `ColorValue`.

```rust,no_check
// crates/ars-core/src/color.rs
//
// The shared color value types live in `ars-core` (flat `color` module,
// re-exported at the crate root) so every color component — ColorSwatch,
// ColorField, ColorArea, ColorSlider, ColorWheel, ColorSwatchPicker, and
// ColorPicker — consumes them as `ars_core::color::{ColorValue, ColorChannel, …}`.

/// A color value stored in HSL with alpha. All other formats are computed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorValue {
    /// Hue in degrees [0, 360).
    pub hue: f64,
    /// Saturation as a fraction [0.0, 1.0].
    pub saturation: f64,
    /// Lightness as a fraction [0.0, 1.0].
    pub lightness: f64,
    /// Alpha as a fraction [0.0, 1.0].
    pub alpha: f64,
}

impl ColorValue {
    /// Create a new `ColorValue` from the given hue, saturation, lightness, and alpha.
    ///
    /// Hue is wrapped into `[0, 360)`; saturation, lightness, and alpha are
    /// clamped to `[0.0, 1.0]`. Non-finite inputs (`NaN`/`inf`) are coerced to
    /// `0.0` so the result is always a valid color.
    pub fn new(hue: f64, saturation: f64, lightness: f64, alpha: f64) -> Self {
        // Coerce non-finite inputs to `0.0` so the type's invariants (finite
        // components, hue in `[0, 360)`, others in `[0, 1]`) hold in release
        // builds too — otherwise `NaN`/`inf` would leak into generated CSS and
        // ARIA. Parsers already reject non-finite user input; this guards the
        // public constructor against programmatic non-finite values.
        let finite = |value: f64| if value.is_finite() { value } else { 0.0 };
        Self {
            hue: finite(hue).rem_euclid(360.0),
            saturation: finite(saturation).clamp(0.0, 1.0),
            lightness: finite(lightness).clamp(0.0, 1.0),
            alpha: finite(alpha).clamp(0.0, 1.0),
        }
    }

    /// Create from an HSL triplet with full alpha.
    pub fn from_hsl(h: f64, s: f64, l: f64) -> Self {
        Self::new(h, s, l, 1.0)
    }

    /// Convert to RGB (0-255 per channel).
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        let h = self.hue / 360.0;
        let s = self.saturation;
        let l = self.lightness;
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;
        let (r1, g1, b1) = match (h * 6.0).floor() as u8 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        (
            ((r1 + m) * 255.0).round() as u8,
            ((g1 + m) * 255.0).round() as u8,
            ((b1 + m) * 255.0).round() as u8,
        )
    }

    /// Convert to RGBA (0-255 per channel, alpha 0-255).
    pub fn to_rgba(&self) -> (u8, u8, u8, u8) {
        let (r, g, b) = self.to_rgb();
        (r, g, b, (self.alpha * 255.0).round() as u8)
    }

    /// Convert to hex string (6-digit or 8-digit with alpha).
    pub fn to_hex(&self, include_alpha: bool) -> String {
        let (r, g, b) = self.to_rgb();
        if include_alpha && self.alpha < 1.0 {
            let a = (self.alpha * 255.0).round() as u8;
            format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
        } else {
            format!("#{:02x}{:02x}{:02x}", r, g, b)
        }
    }

    /// Convert to CSS hsl() or hsla() string.
    pub fn to_css_hsl(&self) -> String {
        if self.alpha < 1.0 {
            format!(
                "hsla({:.0}, {:.1}%, {:.1}%, {:.2})",
                self.hue,
                self.saturation * 100.0,
                self.lightness * 100.0,
                self.alpha,
            )
        } else {
            format!(
                "hsl({:.0}, {:.1}%, {:.1}%)",
                self.hue,
                self.saturation * 100.0,
                self.lightness * 100.0,
            )
        }
    }

    /// Convert to HSB/HSV representation.
    pub fn to_hsb(&self) -> (f64, f64, f64) {
        let l = self.lightness;
        let s = self.saturation;
        let v = l + s * l.min(1.0 - l);
        let sv = if v == 0.0 { 0.0 } else { 2.0 * (1.0 - l / v) };
        (self.hue, sv, v)
    }

    /// Create from an HSB/HSV triplet with full alpha (inverse of `to_hsb`).
    /// Used by ColorPicker so the HSB area/inputs edit HSB saturation × brightness
    /// coherently (HSB→HSL: `l = v·(1 − s/2)`, then recover HSL saturation).
    pub fn from_hsb(hue: f64, saturation: f64, brightness: f64) -> Self {
        let s = saturation.clamp(0.0, 1.0);
        let v = brightness.clamp(0.0, 1.0);
        let l = v * (1.0 - s / 2.0);
        let sl = if l <= 0.0 || l >= 1.0 { 0.0 } else { (v - l) / l.min(1.0 - l) };
        Self::new(hue, sl, l, 1.0)
    }

    /// Create from RGB values (0-255).
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        let r = r as f64 / 255.0;
        let g = g as f64 / 255.0;
        let b = b as f64 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;
        if max == min {
            return Self::new(0.0, 0.0, l, 1.0);
        }
        let d = max - min;
        let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
        let h = if max == r {
            ((g - b) / d + if g < b { 6.0 } else { 0.0 }) * 60.0
        } else if max == g {
            ((b - r) / d + 2.0) * 60.0
        } else {
            ((r - g) / d + 4.0) * 60.0
        };
        Self::new(h, s, l, 1.0)
    }

    /// Parse a hex string ("#rrggbb" or "#rrggbbaa").
    pub fn from_hex(hex: &str) -> Option<Self> {
        // Strip at most one leading `#`. `trim_start_matches` would swallow
        // extra markers, accepting malformed input like `##3366ff`.
        let hex = hex.strip_prefix('#').unwrap_or(hex);

        // Hex digits are ASCII. Reject non-ASCII early so the byte-indexed
        // slices below cannot land on a non-char boundary and panic (a
        // multi-byte string such as `ああ` is exactly 6 bytes).
        if !hex.is_ascii() {
            return None;
        }

        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::from_rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                let mut color = Self::from_rgb(r, g, b);
                color.alpha = a as f64 / 255.0;
                Some(color)
            }
            _ => None,
        }
    }
}

impl Default for ColorValue {
    fn default() -> Self {
        Self::new(0.0, 1.0, 0.5, 1.0) // Pure red, full opacity
    }
}
```

### Contrast Ratio Utility Functions

To assist applications that use `ColorPicker` for theme building or accessible color selection, the following utility functions are provided on `ColorValue`:

```rust
impl ColorValue {
    /// Compute the relative luminance per WCAG 2.1 definition.
    /// Uses the sRGB linearization formula: if C <= 0.04045, C/12.92;
    /// else ((C + 0.055) / 1.055)^2.4. Returns a value in [0.0, 1.0].
    pub fn relative_luminance(&self) -> f64 {
        let (r, g, b) = self.to_rgb();
        fn linearize(c: u8) -> f64 {
            let c = c as f64 / 255.0;
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        }
        0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)
    }

    /// Compute the WCAG 2.1 contrast ratio between two colors.
    /// Returns a value in [1.0, 21.0]. Higher is more contrast.
    pub fn contrast_ratio(&self, other: &ColorValue) -> f64 {
        let l1 = self.relative_luminance();
        let l2 = other.relative_luminance();
        let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
        (lighter + 0.05) / (darker + 0.05)
    }

    /// Check if this color passes WCAG AA contrast against another color.
    /// Normal text: 4.5:1; large text (>= 18pt or >= 14pt bold): 3:1.
    pub fn passes_wcag_aa(&self, other: &ColorValue, large_text: bool) -> bool {
        let ratio = self.contrast_ratio(other);
        if large_text { ratio >= 3.0 } else { ratio >= 4.5 }
    }

    /// Check if this color passes WCAG AAA contrast against another color.
    /// Normal text: 7:1; large text: 4.5:1.
    pub fn passes_wcag_aaa(&self, other: &ColorValue, large_text: bool) -> bool {
        let ratio = self.contrast_ratio(other);
        if large_text { ratio >= 4.5 } else { ratio >= 7.0 }
    }
}
```

**Usage example** (application-level validation):

```rust,no_check
let foreground = color_picker_api.value();
let background = ColorValue::from_hex("#ffffff").unwrap();
let ratio = foreground.contrast_ratio(&background);
let passes = foreground.passes_wcag_aa(&background, false);
// Display: "Contrast ratio: 4.7:1 AA" or "2.1:1 Fails AA"
```

These utilities are provided as helpers on `ColorValue` in `ars-core`. The component itself does not validate or enforce contrast -- this remains the application's responsibility.

### Color Name Parts

```rust
/// Describes a color in human-readable terms for screen readers.
/// Components: lightness modifier, chroma modifier, and hue name.
/// Parts are returned as `String` (not `&'static str`) to support localized labels.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorNameParts {
    /// e.g., "dark", "light", "very dark", "very light", or "" for medium.
    pub lightness: String,
    /// e.g., "vibrant", "pale", "grayish", or "" for moderate.
    pub chroma: String,
    /// e.g., "red", "blue", "cyan-blue", "gray", "white", "black".
    pub hue: String,
}

impl ColorValue {
    /// Returns English color-description parts for accessibility.
    ///
    /// The parts are English keys (e.g. lightness `"dark"`, chroma `"vibrant"`,
    /// hue `"blue"`); a component's `format_name` message maps and reorders them
    /// per locale (English: `"{lightness} {chroma} {hue}"`, Italian:
    /// `"{hue} {chroma} {lightness}"`). Keeping the keys English here decouples
    /// `ColorValue` from any single component's `Messages`.
    ///
    /// Classification runs in OKLCH (perceptually uniform):
    /// 1. Convert HSL -> sRGB -> OKLab -> OKLCH (Björn Ottosson's transform).
    /// 2. Lightness -> 5 levels: very dark (0-0.2), dark (0.2-0.4),
    ///    medium/`""` (0.4-0.6), light (0.6-0.8), very light (>=0.8).
    /// 3. Chroma: grayish (<0.04), moderate/`""` (0.04-0.12), vibrant (>0.12).
    ///    Moderate chroma with light lightness reads as "pale".
    /// 4. Hue angle -> 13 named hues: red, red-orange, orange, yellow-orange,
    ///    yellow, yellow-green, green, cyan-green, cyan, cyan-blue, blue,
    ///    purple, magenta.
    /// 5. Near-zero chroma collapses to "gray"/"white"/"black" by lightness.
    pub fn color_name_parts(&self) -> ColorNameParts {
        let (lightness, chroma, hue) = self.oklch_classify();
        ColorNameParts {
            lightness: lightness.to_string(),
            chroma: chroma.to_string(),
            hue: hue.to_string(),
        }
    }

    /// Convenience: format as an English-order string such as "dark vibrant blue".
    /// Joins the non-empty parts with single spaces.
    pub fn color_name_en(&self) -> String {
        let parts = self.color_name_parts();
        [parts.lightness.as_str(), parts.chroma.as_str(), parts.hue.as_str()]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" ")
    }
}
```

### Supporting Enums and Functions

```rust
/// The color format currently displayed in the text input area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorFormat {
    /// The hex format.
    Hex,
    /// The rgb format.
    Rgb,
    /// The hsl format.
    Hsl,
    /// The hsb format.
    Hsb,
}

impl Default for ColorFormat {
    fn default() -> Self {
        ColorFormat::Hex
    }
}

/// Color space for the picker controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSpace {
    /// The rgb color space.
    Rgb,
    /// The hsl color space.
    Hsl,
    /// The hsb color space.
    Hsb,
    /// The hwb color space.
    Hwb,
}

impl Default for ColorSpace {
    fn default() -> Self {
        ColorSpace::Hsl
    }
}

/// Individual color channel identifier, used by ColorArea, ColorSlider,
/// and ColorPicker for per-channel operations.
///
/// `Hash` is derived so the channel can parameterize a `ComponentPart` variant
/// (e.g. `ColorPicker`'s `Part::ChannelSlider { channel }`), which requires
/// `Eq + Hash`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ColorChannel {
    /// The hue channel.
    #[default]
    Hue,
    /// The saturation channel.
    Saturation,
    /// The lightness channel.
    Lightness,
    /// The brightness channel.
    /// HSB/HSV value component
    Brightness,
    /// The alpha channel.
    Alpha,
    /// The red channel.
    Red,
    /// The green channel.
    Green,
    /// The blue channel.
    Blue,
}

/// Identifies what the user is dragging in the ColorPicker.
/// ColorPicker's area targets two channels at once, while channel sliders
/// target a single channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragTarget {
    /// 2D area: xChannel=Saturation, yChannel=Lightness (fixed in ColorPicker).
    Area,
    /// 1D channel slider.
    Channel(ColorChannel),
}

/// Get the current value of a single channel from a ColorValue.
pub fn channel_value(color: &ColorValue, channel: ColorChannel) -> f64 {
    match channel {
        ColorChannel::Hue => color.hue,
        ColorChannel::Saturation => color.saturation,
        ColorChannel::Lightness => color.lightness,
        ColorChannel::Brightness => color.to_hsb().2,
        ColorChannel::Alpha => color.alpha,
        ColorChannel::Red => color.to_rgb().0 as f64,
        ColorChannel::Green => color.to_rgb().1 as f64,
        ColorChannel::Blue => color.to_rgb().2 as f64,
    }
}

/// Return a new ColorValue with one channel replaced.
pub fn with_channel(color: &ColorValue, channel: ColorChannel, value: f64) -> ColorValue {
    match channel {
        ColorChannel::Hue => ColorValue::new(value, color.saturation, color.lightness, color.alpha),
        ColorChannel::Saturation => ColorValue::new(color.hue, value, color.lightness, color.alpha),
        ColorChannel::Lightness => ColorValue::new(color.hue, color.saturation, value, color.alpha),
        ColorChannel::Brightness => {
            // Convert current HSL to HSB, replace B, convert back
            let (h, s_hsb, _) = color.to_hsb();
            let v = value.clamp(0.0, 1.0);
            let l = v * (1.0 - s_hsb / 2.0);
            let s_hsl = if l == 0.0 || l == 1.0 { 0.0 } else { (v - l) / l.min(1.0 - l) };
            ColorValue::new(h, s_hsl, l, color.alpha)
        }
        ColorChannel::Alpha => ColorValue::new(color.hue, color.saturation, color.lightness, value),
        ColorChannel::Red => {
            let (_, g, b) = color.to_rgb();
            let mut c = ColorValue::from_rgb(value.round() as u8, g, b);
            c.alpha = color.alpha;
            c
        }
        ColorChannel::Green => {
            let (r, _, b) = color.to_rgb();
            let mut c = ColorValue::from_rgb(r, value.round() as u8, b);
            c.alpha = color.alpha;
            c
        }
        ColorChannel::Blue => {
            let (r, g, _) = color.to_rgb();
            let mut c = ColorValue::from_rgb(r, g, value.round() as u8);
            c.alpha = color.alpha;
            c
        }
    }
}

/// (min, max) range for a channel.
pub fn channel_range(channel: ColorChannel) -> (f64, f64) {
    match channel {
        ColorChannel::Hue => (0.0, 360.0),
        ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue => (0.0, 255.0),
        _ => (0.0, 1.0), // Saturation, Lightness, Brightness, Alpha
    }
}

/// Default step size for keyboard adjustment.
pub fn channel_step_default(channel: ColorChannel) -> f64 {
    match channel {
        ColorChannel::Hue => 1.0,        // 1 degree
        ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue => 1.0,
        _ => 0.01,                        // 1% for 0..1 range channels
    }
}

/// Strip a functional notation call: `name(...)` -> inner content.
fn strip_fn_call<'a>(input: &'a str, name: &str) -> Option<&'a str> {
    let stripped = input.strip_prefix(name)?;
    let stripped = stripped.strip_prefix('(')?.strip_suffix(')')?;

    Some(stripped)
}

/// Parse "r, g, b" or "r, g, b, a" inside rgb()/rgba().
fn parse_rgb_args(inner: &str, has_alpha: bool) -> Option<ColorValue> {
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if has_alpha && parts.len() != 4 { return None; }
    if !has_alpha && parts.len() != 3 { return None; }
    let r: u8 = parts[0].parse().ok()?;
    let g: u8 = parts[1].parse().ok()?;
    let b: u8 = parts[2].parse().ok()?;
    let a: f64 = if has_alpha { parts[3].parse().ok()? } else { 1.0 };

    ColorValue::from_rgb(r, g, b).map(|mut c| { c.alpha = a.clamp(0.0, 1.0); c })
}

/// Parse "h, s%, l%" or "h, s%, l%, a" inside hsl()/hsla().
fn parse_hsl_args(inner: &str, has_alpha: bool) -> Option<ColorValue> {
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if has_alpha && parts.len() != 4 { return None; }
    if !has_alpha && parts.len() != 3 { return None; }
    let h: f64 = parts[0].parse().ok()?;
    let s: f64 = parts[1].strip_suffix('%')?.trim().parse::<f64>().ok()? / 100.0;
    let l: f64 = parts[2].strip_suffix('%')?.trim().parse::<f64>().ok()? / 100.0;
    let a: f64 = if has_alpha { parts[3].parse().ok()? } else { 1.0 };

    Some(ColorValue::new(h, s, l, a.clamp(0.0, 1.0)))
}

/// Parse "h, s%, b%" inside hsb() or "h, s%, b%, a" inside hsba().
fn parse_hsb_args(inner: &str, has_alpha: bool) -> Option<ColorValue> {
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    let expected = if has_alpha { 4 } else { 3 };
    if parts.len() != expected { return None; }
    let h: f64 = parts[0].parse().ok()?;
    // Clamp the HSB channels to `0..=1` before the conversion so out-of-range
    // input (e.g. `200%`) doesn't skew lightness/saturation and lose the hue.
    let s: f64 = (parts[1].strip_suffix('%')?.trim().parse::<f64>().ok()? / 100.0).clamp(0.0, 1.0);
    let b: f64 = (parts[2].strip_suffix('%')?.trim().parse::<f64>().ok()? / 100.0).clamp(0.0, 1.0);
    let a: f64 = if has_alpha { parts[3].parse().ok()? } else { 1.0 };
    // Convert HSB -> HSL: lightness = b * (1 - s/2)
    let l = b * (1.0 - s / 2.0);
    let sl = if l > 0.0 && l < 1.0 { (b - l) / l.min(1.0 - l) } else { 0.0 };

    Some(ColorValue::new(h, sl, l, a.clamp(0.0, 1.0)))
}

/// Parse a user-typed string into a ColorValue.
/// Recognizes: "#rrggbb", "#rrggbbaa", "rgb(r,g,b)", "rgba(r,g,b,a)",
/// "hsl(h,s%,l%)", "hsla(h,s%,l%,a)", "hsb(h,s%,b%)".
pub fn parse_color_string(input: &str) -> Option<ColorValue> {
    let trimmed = input.trim();
    if trimmed.starts_with('#') {
        return ColorValue::from_hex(trimmed);
    }
    if let Some(inner) = strip_fn_call(trimmed, "rgba") {
        return parse_rgb_args(inner, true);
    }
    if let Some(inner) = strip_fn_call(trimmed, "rgb") {
        return parse_rgb_args(inner, false);
    }
    if let Some(inner) = strip_fn_call(trimmed, "hsla") {
        return parse_hsl_args(inner, true);
    }
    if let Some(inner) = strip_fn_call(trimmed, "hsl") {
        return parse_hsl_args(inner, false);
    }
    if let Some(inner) = strip_fn_call(trimmed, "hsba") {
        return parse_hsb_args(inner, true);
    }
    if let Some(inner) = strip_fn_call(trimmed, "hsb") {
        return parse_hsb_args(inner, false);
    }

    None
}

/// Format a ColorValue as a string in the given format.
pub fn format_color_string(color: &ColorValue, format: ColorFormat) -> String {
    match format {
        ColorFormat::Hex => color.to_hex(color.alpha < 1.0),
        ColorFormat::Rgb => {
            let (r, g, b) = color.to_rgb();
            if color.alpha < 1.0 {
                format!("rgba({}, {}, {}, {:.2})", r, g, b, color.alpha)
            } else {
                format!("rgb({}, {}, {})", r, g, b)
            }
        }
        ColorFormat::Hsl => color.to_css_hsl(),
        ColorFormat::Hsb => {
            let (h, s, b) = color.to_hsb();
            // Emit `hsba(...)` for translucent colors so alpha round-trips
            // through the parser instead of being silently dropped to opaque.
            if color.alpha < 1.0 {
                format!("hsba({:.0}, {:.1}%, {:.1}%, {:.2})", h, s * 100.0, b * 100.0, color.alpha)
            } else {
                format!("hsb({:.0}, {:.1}%, {:.1}%)", h, s * 100.0, b * 100.0)
            }
        }
    }
}
```

### Supported Color Spaces and Conversion

**Minimum supported color spaces**: HSL, HSB (HSV), RGB, HEX.

| Space | To                                | From                              | Notes                     |
| ----- | --------------------------------- | --------------------------------- | ------------------------- |
| HSL   | (native storage)                  | `ColorValue::new(h, s, l, a)`     | Canonical internal format |
| RGB   | `to_rgb() -> (u8, u8, u8)`        | `from_rgb(r, g, b) -> Self`       | 0-255 per channel         |
| HEX   | `to_hex(include_alpha) -> String` | `from_hex(hex) -> Option<Self>`   | `#rrggbb` or `#rrggbbaa`  |
| HSB   | `to_hsb() -> (f64, f64, f64)`     | via `with_channel(Brightness, v)` | Hue shared with HSL       |
| CSS   | `to_css_hsl() -> String`          | --                                | `hsl()`/`hsla()` string   |

Color space conversion rules:

1. Out-of-gamut values are clamped to the target color space's valid range during conversion (e.g., HSL saturation > 100% clamped to 100%).
2. Channel values are rounded to the precision appropriate for the color space: RGB channels to integers (0-255), HSL/HSB hue to 1 decimal place, saturation/lightness to integers.
3. When switching color spaces in the UI, the displayed value is the conversion result of the current color -- no rounding trip-loss accumulation (convert from stored high-precision internal representation each time).

## 1. State Machine

### 1.1 States

```rust
/// The states for the ColorPicker component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Picker is closed (trigger visible, content hidden).
    Closed,
    /// Picker is open, user is not actively dragging.
    Open,
    /// User is dragging a thumb (area or channel slider).
    Dragging {
        /// The target of the drag.
        target: DragTarget,
    },
}
```

### 1.2 Events

```rust
/// The events for the ColorPicker component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Open the picker popover.
    Open,
    /// Close the picker popover.
    Close,
    /// Toggle open/closed.
    Toggle,
    /// User started dragging a thumb (area or channel slider).
    DragStart {
        /// The target of the drag.
        target: DragTarget,
        /// The x coordinate.
        x: f64,
        /// The y coordinate.
        y: f64,
    },
    /// User is moving while dragging.
    DragMove {
        /// The x coordinate.
        x: f64,
        /// The y coordinate.
        y: f64,
    },
    /// User released the drag.
    DragEnd,
    /// Set the color value directly (e.g., from text input).
    SetColor(ColorValue),
    /// Set individual channel value (from channel input).
    SetChannel {
        /// The channel.
        channel: ColorChannel,
        /// The value.
        value: f64,
    },
    /// Switch the displayed format.
    SetFormat(ColorFormat),
    /// Switch the active color space. Triggers value recomputation.
    ChangeColorSpace(ColorSpace),
    /// Eyedropper result.
    EyedropperResult(Option<ColorValue>),
    /// Eyedropper requested.
    EyedropperRequest,
    /// Focus entered a part.
    Focus {
        /// The part.
        part: &'static str,
    },
    /// Focus left a part.
    Blur {
        /// The part.
        part: &'static str,
    },
    /// Keyboard channel adjustment.
    ChannelIncrement {
        /// The channel.
        channel: ColorChannel,
        /// The step.
        step: f64,
    },
    /// Keyboard channel adjustment (decrement).
    ChannelDecrement {
        /// The channel.
        channel: ColorChannel,
        /// The step.
        step: f64,
    },
    /// Keyboard adjustment of the 2D area's horizontal (saturation) axis by a
    /// signed fraction. Space-aware (HSB saturation in HSB, else HSL): the area
    /// is a 2D control, not a single channel, so it does not reuse
    /// `ChannelIncrement`/`Decrement`.
    AreaXStep(f64),
    /// Keyboard adjustment of the 2D area's vertical axis by a signed fraction
    /// (brightness in HSB, lightness otherwise).
    AreaYStep(f64),
    /// Close on interact outside (suppressed while dragging).
    CloseOnInteractOutside,
    /// Close on escape.
    CloseOnEscape,
    /// Adapter-reported browser EyeDropper API availability. Sent in response to
    /// the [`Effect::DetectEyedropper`] intent emitted when the picker opens.
    SetEyedropperSupported(bool),
    /// Controlled-value sync from the parent after `Service::set_props`.
    SyncValue(Option<ColorValue>),
    /// Refresh cached output props after `Service::set_props`.
    SetProps,
}
```

`Event::EyedropperRequest` only produces the `Effect::InvokeEyedropper` intent
when `eyedropper_supported` is `true` and the component is not read-only.
Controlled `open` changes are driven by `Open`/`Close` (see
[§1.7](#17-full-machine-implementation) `on_props_changed`), so no dedicated
open-sync event is needed.

### 1.3 Context

```rust
/// The context for the `ColorPicker` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The current color value (controlled or uncontrolled).
    pub value: Bindable<ColorValue>,
    /// The currently displayed format in the input area.
    pub format: ColorFormat,
    /// Whether the picker popover is open. A plain `bool` mirroring `State`
    /// (the source of truth); the controlled-vs-uncontrolled `open` distinction
    /// is resolved by `on_props_changed` emitting `Open`/`Close`, so there is no
    /// separate controlled slot to drift out of sync with `State`.
    pub open: bool,
    /// The unwrapped hue in degrees `[0, 360]` for the hue channel slider.
    /// `ColorValue` normalizes hue into `[0, 360)`, so reading hue back from the
    /// color collapses the 360° endpoint onto 0°; tracking it here keeps the hue
    /// slider's thumb position and `aria-valuenow` at the right-hand endpoint
    /// (the same technique the standalone `ColorSlider` uses).
    pub hue_value: f64,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is read-only.
    pub readonly: bool,
    /// Whether to close the picker when the user interacts outside.
    pub close_on_interact_outside: bool,
    /// Whether to close the picker when Escape is pressed.
    pub close_on_escape: bool,
    /// Whether to show the alpha channel.
    pub show_alpha: bool,
    /// Whether the EyeDropper API is available in the browser.
    pub eyedropper_supported: bool,
    /// Currently focused part name (or None).
    pub focused_part: Option<&'static str>,
    /// Positioning options for the popover.
    pub positioning: PositioningOptions,
    /// Keyboard step for channel adjustments.
    pub channel_step: f64,
    /// Large step (Shift+Arrow or PageUp/PageDown).
    pub channel_large_step: f64,
    /// Active color space for the picker controls.
    pub color_space: ColorSpace,
    /// Preset swatch colors rendered in the swatch group.
    pub swatches: Vec<ColorValue>,
    /// Text direction for RTL-aware keyboard navigation.
    pub dir: Direction,
    /// Locale for internationalized messages.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Component instance IDs. Part ids are derived on demand via
    /// `ids.part("trigger")` / `ids.item("channel", &index)` — the same
    /// convention as every sibling color component — rather than precomputed
    /// into the context. The string ids exist purely for ARIA wiring and
    /// hydration-stable `id` attributes, never as a substitute for live handles.
    pub ids: ComponentIds,
}

impl Context {
    /// Returns the channels available in the current color space, in display
    /// order. `Alpha` is appended whenever `show_alpha` is set, so an adapter
    /// that renders one input per returned channel also renders the alpha input
    /// the `show_alpha`/alpha-slider contract promises. (HWB maps onto the HSL
    /// channel triplet for the numeric inputs.) Returns one of a fixed set of
    /// `&'static` slices keyed by `(color_space, show_alpha)`.
    pub const fn channels(&self) -> &'static [ColorChannel] { /* per (space, show_alpha) */ }

    /// The vertical channel the 2D area edits in the current color space:
    /// brightness for HSB, lightness for HSL / RGB / HEX. (The horizontal axis is
    /// always saturation; the hue and alpha sliders are space-independent.)
    pub const fn area_y_channel(&self) -> ColorChannel {
        match self.color_space {
            ColorSpace::Hsb => ColorChannel::Brightness,
            _ => ColorChannel::Lightness,
        }
    }
}
```

### 1.4 Props

```rust
/// The props for the `ColorPicker` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Controlled value. When `Some`, the component is controlled.
    pub value: Option<ColorValue>,
    /// Default value for uncontrolled mode.
    pub default_value: ColorValue,
    /// Controlled open state.
    pub open: Option<bool>,
    /// Default open state for uncontrolled mode.
    pub default_open: bool,
    /// Disabled state.
    pub disabled: bool,
    /// Read-only state.
    pub readonly: bool,
    /// Close on interact outside the popover.
    pub close_on_interact_outside: bool,
    /// Close on Escape.
    pub close_on_escape: bool,
    /// Show the alpha channel slider and input.
    pub show_alpha: bool,
    /// Initial format for the text inputs.
    pub default_format: ColorFormat,
    /// Positioning options for the popover.
    pub positioning: PositioningOptions,
    /// Step size for keyboard channel adjustment.
    pub channel_step: f64,
    /// Large step size for keyboard channel adjustment (Shift+Arrow).
    pub channel_large_step: f64,
    /// Color space for the picker controls.
    /// Default: `ColorSpace::Hsl`.
    pub color_space: ColorSpace,
    /// Preset swatch colors rendered in the swatch group. Resolved by
    /// `Part::Swatch { index }` / `Api::swatch_attrs(index)`.
    pub swatches: Vec<ColorValue>,
    /// Text direction for RTL-aware keyboard navigation and layout.
    /// Default: `Direction::Ltr`.
    pub dir: Direction,
    /// Name attribute for the hidden form input.
    pub name: Option<String>,
    /// Component instance ID.
    pub id: String,
    /// Callback fired once when a drag interaction ends (pointerup on the area or
    /// a channel slider). Unlike continuous change callbacks, this fires once at
    /// the end of the gesture. Use for expensive operations like saving to a
    /// server. The trait-object alias keeps the callback `Send + Sync`.
    pub on_change_end: Option<Callback<dyn Fn(ColorValue) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            value: None,
            default_value: ColorValue::default(),
            open: None,
            default_open: false,
            disabled: false,
            readonly: false,
            close_on_interact_outside: true,
            close_on_escape: true,
            show_alpha: true,
            default_format: ColorFormat::Hex,
            positioning: PositioningOptions {
                placement: Placement::BottomStart,
                offset: Offset { main_axis: 4.0, cross_axis: 0.0 },
                ..Default::default()
            },
            channel_step: 1.0,
            channel_large_step: 10.0,
            color_space: ColorSpace::default(),
            swatches: Vec::new(),
            dir: Direction::Ltr,
            name: None,
            id: String::new(),
            on_change_end: None,
        }
    }
}
```

### 1.5 Guards

```rust
/// Whether the component is disabled.
fn is_disabled(ctx: &Context, _props: &Props) -> bool {
    ctx.disabled
}

/// Whether the component is read-only.
fn is_readonly(ctx: &Context, _props: &Props) -> bool {
    ctx.readonly
}

/// Whether the component is open.
fn is_open(ctx: &Context, _props: &Props) -> bool {
    ctx.open
}
```

### 1.6 Color Space Switching

`ColorPicker` supports runtime color space switching via `Event::ChangeColorSpace(ColorSpace)`.

**Transition Behavior** for `ChangeColorSpace(new_space)`:

1. **Update context**: Set `ctx.color_space = new_space`. The stored `ColorValue` is unchanged — the same color is simply presented through the new space's parameters.
2. **Remap the numeric inputs**: `Context::channels()` returns the new space's channel set (RGB→`[R, G, B]`, HSL→`[H, S, L]`, HSB→`[H, S, B]`, plus `Alpha` when `show_alpha`), so the per-channel text inputs re-label and re-bind to the new channels.
3. **Remap the 2D area's vertical axis**: the area is always the visual saturation × {lightness | brightness} square of the current hue — `Context::area_y_channel()` returns brightness in HSB and lightness in HSL/RGB/HEX. The horizontal axis is always saturation, and the hue and alpha sliders are space-independent (always hue/alpha). The area is **not** remapped to arbitrary channel pairs: RGB/HEX have no perceptually-navigable hue square (they are edited through the numeric inputs), so they reuse the HSL square as the visual picker. This is the standard Ark UI / React Aria picker model.
4. **Live region announcement**: Emit an `aria-live="polite"` announcement: `"Switched to {space} color space"` (e.g., "Switched to HSL color space"). This informs screen reader users that the control layout has changed.

A controlled `color_space` prop change is routed through this same `Event::ChangeColorSpace` from `on_props_changed`, so a prop-driven switch announces and remaps identically to a runtime one — and `SetProps` never touches `color_space`, so an unrelated prop update cannot revert a runtime switch.

**UI Affordance**: The color space selector (button group, dropdown, or segmented control) emits `Event::ChangeColorSpace(selected_space)` when the user selects a different space. The spec does not prescribe a specific UI pattern -- adapters choose the appropriate control.

**Supported Spaces**: All spaces in the `ColorSpace` enum (RGB, HSL, HSB/HSV, HEX) are switchable at runtime. The conversion between spaces is lossless for the sRGB gamut (minor floating-point rounding is acceptable).

### 1.7 Full Machine Implementation

The machine follows the canonical sibling conventions: controlled/uncontrolled
`value` and `open` are held in `Bindable`s, side effects are emitted as a typed
`Effect` enum (the agnostic core never touches the DOM or browser APIs), and
parent prop changes flow back through `on_props_changed`. Pointer drag is
**adapter-driven**: the adapter measures the dragged surface and sends
already-normalized `(x, y)` in `0..=1` via `DragStart`/`DragMove`/`DragEnd`,
exactly as `ColorArea`/`ColorSlider` do, so no element measurement happens inside
a core effect closure.

```rust
/// Typed identifier for the named side effects the machine emits. Adapters
/// dispatch on these exhaustively; the core never performs the work itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Invoke `Props::on_change_end` with the final color (fired on `DragEnd`).
    ChangeEnd,
    /// Attach the click-outside listener (fired on `Closed → Open` and a
    /// non-`Closed` initial mount). The adapter dispatches
    /// `Event::CloseOnInteractOutside` when an outside interaction occurs.
    AttachClickOutside,
    /// Detach the click-outside listener (fired on `Open → Closed`).
    DetachClickOutside,
    /// Detect browser EyeDropper support (fired on `Closed → Open` and a
    /// non-`Closed` initial mount). The adapter performs the
    /// `"EyeDropper" in window` check and reports the result via
    /// `Event::SetEyedropperSupported`.
    DetectEyedropper,
    /// Open the browser EyeDropper (fired on `EyedropperRequest`). The adapter
    /// calls `EyeDropper.open()` from the originating user gesture and reports
    /// the outcome via `Event::EyedropperResult`.
    InvokeEyedropper,
    /// Announce the active color-space change via an `aria-live` region. The
    /// adapter reads the text from `Api::color_space_announcement`.
    AnnounceColorSpace,
}

/// Apply an adapter-normalized pointer position to the color value for the given
/// drag target. `Area` maps x→saturation `[0, 1]` and y→the area's vertical
/// channel `[1, 0]` (brightness in HSB, lightness otherwise); `Channel` maps x
/// across the channel's full range. Bases the new color on the *pending* value
/// so a controlled drag-in-flight accumulates.
fn apply_drag_position(ctx: &mut Context, target: DragTarget, x: f64, y: f64) {
    match target {
        DragTarget::Area => set_area(ctx, x, 1.0 - y),
        DragTarget::Channel(channel) => {
            let (min, max) = channel_range(channel);
            let value = min + x.clamp(0.0, 1.0) * (max - min);
            set_channel_value(ctx, channel, value);
        }
    }
}

/// Current area coordinates `(saturation, y)` in `0..=1`, interpreted in the
/// active space: HSB saturation × brightness for HSB, HSL saturation × lightness
/// otherwise.
fn area_axes(ctx: &Context) -> (f64, f64) {
    let color = ctx.value.pending();
    if ctx.color_space == ColorSpace::Hsb {
        let (_, s, v) = color.to_hsb();
        (s, v)
    } else {
        (color.saturation, color.lightness)
    }
}

/// Write the area coordinates back to the pending color, space-aware so HSB edits
/// set both axes together via `from_hsb` (no HSL-sat/HSB-bright mixing). Hue/alpha
/// preserved.
fn set_area(ctx: &mut Context, saturation: f64, y: f64) {
    let current = *ctx.value.pending();
    let s = saturation.clamp(0.0, 1.0);
    let y = y.clamp(0.0, 1.0);
    let next = if ctx.color_space == ColorSpace::Hsb {
        let mut color = ColorValue::from_hsb(current.hue, s, y);
        color.alpha = current.alpha;
        color
    } else {
        let updated = with_channel(&current, ColorChannel::Saturation, s);
        with_channel(&updated, ColorChannel::Lightness, y)
    };
    ctx.value.set(next);
}

/// Set one channel, space-aware: hue also updates the unwrapped `hue_value` (360°
/// endpoint), Saturation-in-HSB is HSB saturation (via `from_hsb`, preserving
/// brightness) so the numeric S input matches the HSB area, others go through
/// `with_channel`.
fn set_channel_value(ctx: &mut Context, channel: ColorChannel, value: f64) {
    let base = *ctx.value.pending();
    if channel == ColorChannel::Saturation && ctx.color_space == ColorSpace::Hsb {
        let (hue, _, brightness) = base.to_hsb();
        let mut next = ColorValue::from_hsb(hue, value, brightness);
        next.alpha = base.alpha;
        ctx.value.set(next);
        return;
    }
    ctx.value.set(with_channel(&base, channel, value));
    if channel == ColorChannel::Hue {
        ctx.hue_value = value.clamp(0.0, 360.0);
    }
}

/// Re-derive the cached unwrapped hue after an external full-color set.
fn sync_hue_from_color(ctx: &mut Context) { ctx.hue_value = ctx.value.pending().hue; }

/// Current value of `channel` for keyboard stepping / numeric display — unwrapped
/// `hue_value` for hue, HSB saturation for Saturation-in-HSB, live value otherwise.
fn channel_current(ctx: &Context, channel: ColorChannel) -> f64 {
    match channel {
        ColorChannel::Hue => ctx.hue_value,
        ColorChannel::Saturation if ctx.color_space == ColorSpace::Hsb => {
            ctx.value.pending().to_hsb().1
        }
        _ => channel_value(ctx.value.pending(), channel),
    }
}

/// Invoke `Props::on_change_end` with the pending (drag-staged) color.
fn change_end_effect() -> PendingEffect<Machine> {
    PendingEffect::new(Effect::ChangeEnd, |ctx: &Context, props: &Props, _send| {
        if let Some(callback) = &props.on_change_end {
            callback(*ctx.value.pending());
        }
        no_cleanup()
    })
}

/// The named effects produced by the open lifecycle. Shared by `open_plan`
/// (`Closed → Open`) and `Machine::initial_effects` (booted-open) so the two
/// entry points stay in lock-step.
fn open_lifecycle_effects() -> [PendingEffect<Machine>; 2] {
    [
        PendingEffect::named(Effect::AttachClickOutside),
        PendingEffect::named(Effect::DetectEyedropper),
    ]
}

fn open_plan() -> TransitionPlan<Machine> {
    let mut plan = TransitionPlan::to(State::Open).apply(|ctx: &mut Context| ctx.open = true);
    for effect in open_lifecycle_effects() {
        plan = plan.with_effect(effect);
    }
    plan
}

fn close_plan() -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Closed)
        .apply(|ctx: &mut Context| ctx.open = false)
        .with_effect(PendingEffect::named(Effect::DetachClickOutside))
}

/// Whether any context-backed non-`value`/`open` prop changed (drives `SetProps`).
fn context_relevant_props_changed(old: &Props, new: &Props) -> bool {
    old.disabled != new.disabled
        || old.readonly != new.readonly
        || old.close_on_interact_outside != new.close_on_interact_outside
        || old.close_on_escape != new.close_on_escape
        || old.show_alpha != new.show_alpha
        // `color_space` is intentionally excluded — it is owned by
        // `ChangeColorSpace` (a prop change routes through that event in
        // `on_props_changed`) so `SetProps` cannot clobber a runtime switch.
        || old.swatches != new.swatches
        || old.dir != new.dir
        || old.positioning != new.positioning
        || (old.channel_step - new.channel_step).abs() > f64::EPSILON
        || (old.channel_large_step - new.channel_large_step).abs() > f64::EPSILON
}

/// The machine for the `ColorPicker` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let value = match props.value {
            Some(color) => Bindable::controlled(color),
            None => Bindable::uncontrolled(props.default_value),
        };
        let open = props.open.unwrap_or(props.default_open);
        let state = if open { State::Open } else { State::Closed };
        let hue_value = value.get().hue;

        (state, Context {
            value,
            open,
            hue_value,
            format: props.default_format,
            disabled: props.disabled,
            readonly: props.readonly,
            close_on_interact_outside: props.close_on_interact_outside,
            close_on_escape: props.close_on_escape,
            show_alpha: props.show_alpha,
            eyedropper_supported: false, // adapter reports via SetEyedropperSupported
            focused_part: None,
            positioning: props.positioning.clone(),
            channel_step: props.channel_step,
            channel_large_step: props.channel_large_step,
            color_space: props.color_space,
            swatches: props.swatches.clone(),
            dir: props.dir,
            locale: env.locale.clone(),
            messages: messages.clone(),
            ids: ComponentIds::from_id(&props.id),
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // A disabled picker ignores user interaction but still tracks focus and
        // accepts parent-driven syncs. Controlled `open` changes arrive as
        // `Open`/`Close`, so those pass through too — otherwise a disabled,
        // controlled picker could never be opened/closed by its parent.
        if ctx.disabled {
            match event {
                Event::Open | Event::Close
                // `DragEnd` passes through so a drag in flight when the parent
                // disables the control still terminates cleanly (fires
                // `on_change_end`, clears `data-ars-dragging`), like ColorArea.
                | Event::DragEnd
                // A controlled `color_space` change arrives as `ChangeColorSpace`
                // (SetProps excludes color_space), so it must pass through or a
                // disabled picker stays stuck on the old channel set.
                | Event::ChangeColorSpace(_)
                | Event::Focus { .. } | Event::Blur { .. }
                | Event::SyncValue(_) | Event::SetProps => {}
                _ => return None,
            }
        }

        match (state, event) {
            (State::Closed, Event::Open | Event::Toggle) => Some(open_plan()),

            // An explicit close is honored from `Dragging` too (parent-controlled
            // `open: false`, `Api::close()`, `Toggle`, Escape): abandon the drag
            // and close rather than get stuck open. Interact-outside stays
            // suppressed during pointer capture (see the dedicated arm below).
            (State::Open | State::Dragging { .. }, Event::Close | Event::Toggle) => {
                Some(close_plan())
            }

            (State::Open, Event::CloseOnInteractOutside) if ctx.close_on_interact_outside => {
                Some(close_plan())
            }
            // Interact-outside is suppressed during a drag (pointer capture active).
            (State::Dragging { .. }, Event::CloseOnInteractOutside) => None,
            (State::Open | State::Dragging { .. }, Event::CloseOnEscape)
                if ctx.close_on_escape => Some(close_plan()),

            (State::Open, Event::DragStart { target, x, y }) => {
                if ctx.readonly { return None; }
                let (target, x, y) = (*target, *x, *y);
                Some(TransitionPlan::to(State::Dragging { target })
                    .apply(move |ctx| apply_drag_position(ctx, target, x, y)))
            }
            (State::Dragging { target }, Event::DragMove { x, y }) => {
                if ctx.readonly { return None; }
                let (target, x, y) = (*target, *x, *y);
                Some(TransitionPlan::context_only(move |ctx| apply_drag_position(ctx, target, x, y)))
            }
            (State::Dragging { .. }, Event::DragEnd) => {
                Some(TransitionPlan::to(State::Open).with_effect(change_end_effect()))
            }

            (_, Event::SetColor(color)) => {
                if ctx.readonly { return None; }
                let color = *color;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(color);
                    sync_hue_from_color(ctx);
                }))
            }
            (State::Open, Event::SetChannel { channel, value }) => {
                if ctx.readonly { return None; }
                let (channel, value) = (*channel, *value);
                Some(TransitionPlan::context_only(move |ctx| set_channel_value(ctx, channel, value)))
            }
            (_, Event::SetFormat(format)) => {
                let format = *format;
                Some(TransitionPlan::context_only(move |ctx| ctx.format = format))
            }
            (_, Event::ChangeColorSpace(new_space)) => {
                let new_space = *new_space;
                Some(TransitionPlan::context_only(move |ctx| ctx.color_space = new_space)
                    .with_effect(PendingEffect::named(Effect::AnnounceColorSpace)))
            }

            (State::Open, Event::ChannelIncrement { channel, step }) => {
                if ctx.readonly { return None; }
                let (channel, step) = (*channel, *step);
                Some(TransitionPlan::context_only(move |ctx| {
                    let (_, max) = channel_range(channel);
                    set_channel_value(ctx, channel, (channel_current(ctx, channel) + step).min(max));
                }))
            }
            (State::Open, Event::ChannelDecrement { channel, step }) => {
                if ctx.readonly { return None; }
                let (channel, step) = (*channel, *step);
                Some(TransitionPlan::context_only(move |ctx| {
                    let (min, _) = channel_range(channel);
                    set_channel_value(ctx, channel, (channel_current(ctx, channel) - step).max(min));
                }))
            }
            // The 2D area steps its own space-aware axes (saturation × lightness
            // or brightness) rather than reusing the channel events.
            (State::Open, Event::AreaXStep(delta)) => {
                if ctx.readonly { return None; }
                let delta = *delta;
                Some(TransitionPlan::context_only(move |ctx| {
                    let (s, y) = area_axes(ctx);
                    set_area(ctx, s + delta, y);
                }))
            }
            (State::Open, Event::AreaYStep(delta)) => {
                if ctx.readonly { return None; }
                let delta = *delta;
                Some(TransitionPlan::context_only(move |ctx| {
                    let (s, y) = area_axes(ctx);
                    set_area(ctx, s, y + delta);
                }))
            }

            (State::Open, Event::EyedropperRequest) => {
                if !ctx.eyedropper_supported || ctx.readonly { return None; }
                Some(TransitionPlan::context_only(|_ctx| {})
                    .with_effect(PendingEffect::named(Effect::InvokeEyedropper)))
            }
            (_, Event::EyedropperResult(Some(color))) => {
                if ctx.readonly { return None; }
                let color = *color;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(color);
                    sync_hue_from_color(ctx);
                }))
            }
            (_, Event::SetEyedropperSupported(supported)) => {
                let supported = *supported;
                Some(TransitionPlan::context_only(move |ctx| ctx.eyedropper_supported = supported))
            }

            (_, Event::Focus { part }) => {
                let part = *part;
                Some(TransitionPlan::context_only(move |ctx| ctx.focused_part = Some(part)))
            }
            (_, Event::Blur { .. }) => {
                Some(TransitionPlan::context_only(|ctx| ctx.focused_part = None))
            }

            (_, Event::SyncValue(value)) => {
                let value = *value;
                Some(TransitionPlan::context_only(move |ctx| {
                    if let Some(color) = value { ctx.value.set(color); }
                    ctx.value.sync_controlled(value);
                    sync_hue_from_color(ctx);
                }))
            }
            (_, Event::SetProps) => {
                let props = props.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.disabled = props.disabled;
                    ctx.readonly = props.readonly;
                    ctx.close_on_interact_outside = props.close_on_interact_outside;
                    ctx.close_on_escape = props.close_on_escape;
                    ctx.show_alpha = props.show_alpha;
                    // `color_space` is owned by `ChangeColorSpace`, not synced here.
                    ctx.swatches = props.swatches;
                    ctx.dir = props.dir;
                    ctx.positioning = props.positioning;
                    ctx.channel_step = props.channel_step;
                    ctx.channel_large_step = props.channel_large_step;
                }))
            }

            _ => None,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        // The id is baked into Context::ids (and every aria-* relationship that
        // points at it) at init; allowing it to change would break ARIA wiring.
        assert_eq!(old.id, new.id, "color_picker::Props.id must remain stable after init");

        let mut events = Vec::new();
        // A controlled `open` flip drives the same Open/Close transition the user
        // would, so the lifecycle effects fire identically.
        if let (was, Some(now)) = (old.open, new.open) && was != Some(now) {
            events.push(if now { Event::Open } else { Event::Close });
        }
        if old.value != new.value {
            events.push(Event::SyncValue(new.value));
        }
        // A controlled `color_space` prop change routes through the same event a
        // runtime switch uses, so it announces/remaps consistently and SetProps
        // never has to touch `color_space` (which would clobber a runtime switch).
        if old.color_space != new.color_space {
            events.push(Event::ChangeColorSpace(new.color_space));
        }
        if context_relevant_props_changed(old, new) {
            events.push(Event::SetProps);
        }
        events
    }

    fn initial_effects(
        state: &Self::State,
        _context: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        // A `default_open`/controlled-open boot returns `State::Open` from `init`,
        // so the `Closed → Open` plan never runs. Mirror its lifecycle effects so
        // adapters drive identical wiring on first mount via `take_initial_effects`.
        if matches!(state, State::Open) {
            open_lifecycle_effects().into_iter().collect()
        } else {
            Vec::new()
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.8 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "color-picker"]
pub enum Part {
    Root,
    Label,
    Control,
    Trigger,
    Content,
    Area,
    AreaThumb,
    /// A channel slider container, parameterized by channel.
    ChannelSlider { channel: ColorChannel },
    /// A channel slider thumb, parameterized by channel.
    ChannelSliderThumb { channel: ColorChannel },
    AlphaSlider,
    SwatchGroup,
    /// A preset swatch button. The color is resolved from `Context::swatches`
    /// at `index`; `ColorValue` is not `Eq`/`Hash` (it holds `f64`s), which
    /// `ComponentPart` requires, so the variant carries only the index — the
    /// same pattern as `color_swatch_picker::Part::Item`.
    Swatch { index: usize },
    FormatSelect,
    /// A channel text input, parameterized by channel and display index.
    ChannelInput { channel: ColorChannel, index: usize },
    HexInput,
    EyeDropperTrigger,
    HiddenInput,
}

/// The connect API for the `ColorPicker` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Api<'_> {
    // --- Computed state ---

    /// Whether the popover is open.
    pub const fn is_open(&self) -> bool {
        !matches!(self.state, State::Closed)
    }

    /// Whether a thumb is currently being dragged.
    pub const fn is_dragging(&self) -> bool {
        matches!(self.state, State::Dragging { .. })
    }

    /// The current color value (the pending value, so controlled drags-in-flight
    /// are reflected).
    pub fn value(&self) -> &ColorValue {
        self.ctx.value.pending()
    }

    /// The current color value formatted as a string in the active format.
    pub fn value_as_string(&self) -> String {
        let color = self.ctx.value.pending();
        match self.ctx.format {
            ColorFormat::Hex => color.to_hex(self.ctx.show_alpha),
            // `to_css_hsl()` emits `hsla(...)` when alpha < 1; suppress alpha when
            // alpha controls are hidden so HSL matches Hex/RGB and the hidden value.
            ColorFormat::Hsl if self.ctx.show_alpha && color.alpha < 1.0 => color.to_css_hsl(),
            ColorFormat::Hsl => format!(
                "hsl({:.0}, {:.1}%, {:.1}%)",
                color.hue, color.saturation * 100.0, color.lightness * 100.0
            ),
            ColorFormat::Rgb => {
                let (r, g, b) = color.to_rgb();
                if self.ctx.show_alpha && color.alpha < 1.0 {
                    format!("rgba({r}, {g}, {b}, {:.2})", color.alpha)
                } else {
                    format!("rgb({r}, {g}, {b})")
                }
            }
            // `hsba(...)` for a translucent color (matching the canonical
            // `format_color_string`) so the text round-trips; `hsb(...)` when
            // opaque or alpha is hidden.
            ColorFormat::Hsb if self.ctx.show_alpha && color.alpha < 1.0 => {
                let (h, s, b) = color.to_hsb();
                format!("hsba({h:.0}, {:.1}%, {:.1}%, {:.2})", s * 100.0, b * 100.0, color.alpha)
            }
            ColorFormat::Hsb => {
                let (h, s, b) = color.to_hsb();
                format!("hsb({h:.0}, {:.1}%, {:.1}%)", s * 100.0, b * 100.0)
            }
        }
    }

    /// The active text format.
    pub const fn format(&self) -> ColorFormat { self.ctx.format }

    /// The active color space.
    pub const fn color_space(&self) -> ColorSpace { self.ctx.color_space }

    /// A human-readable name for the current color (e.g. `"dark vibrant blue"`).
    pub fn color_name(&self) -> String {
        (self.ctx.messages.color_name)(self.ctx.value.pending(), &self.ctx.locale)
    }

    /// The debounced `aria-live` announcement for the current color (see §3.3).
    pub fn color_announcement(&self) -> String {
        (self.ctx.messages.color_announcement)(self.ctx.value.pending(), self.ctx.format, &self.ctx.locale)
    }

    /// The `aria-live` announcement for the active color space, used by the
    /// `Effect::AnnounceColorSpace` adapter handler.
    pub fn color_space_announcement(&self) -> String {
        (self.ctx.messages.color_space_switched)(&format!("{:?}", self.ctx.color_space), &self.ctx.locale)
    }

    // --- Imperative actions ---

    pub fn open(&self) { (self.send)(Event::Open); }
    pub fn close(&self) { (self.send)(Event::Close); }
    pub fn set_value(&self, color: ColorValue) { (self.send)(Event::SetColor(color)); }
    pub fn set_format(&self, format: ColorFormat) { (self.send)(Event::SetFormat(format)); }
}
```

Each `*_attrs` method emits the part's `data-ars-scope`/`data-ars-part` tokens
(via `Part::data_attrs()`) plus the ARIA/`data-ars-*` attributes and CSS custom
properties listed in [§2](#2-anatomy) and [§3](#3-accessibility). Element ids are
derived on demand from `ctx.ids` — `ids.part("trigger")`, `ids.part("content")`,
`ids.part("area-thumb")`, `ids.part("hue-slider")`, `ids.part("alpha-slider")`,
`ids.part("label")`, `ids.part("format-select")`, and `ids.item("channel", &index)`
for the channel inputs — rather than precomputed into the context.

Highlights of the part attribute surface:

- **`trigger_attrs`** — `type="button"` (so a real `<button>` in a form toggles
  rather than submits), `aria-haspopup="dialog"`, `aria-expanded`,
  `aria-controls` (content id), `aria-labelledby` (label id), and `aria-label`
  from `messages.trigger_label`; `aria-disabled="true"` + `data-ars-disabled` when
  disabled.
- **`content_attrs`** — `role="dialog"`, `aria-labelledby` (label id),
  `data-ars-state` = `open`/`closed`.
- **`area_attrs`** — `role="group"` plus the `--ars-color-picker-area-bg` hue
  backdrop custom property.
- **`area_thumb_attrs`** — `role="application"`, `aria-roledescription`,
  `aria-label`, a composed `aria-valuetext` of saturation + the area's vertical
  axis (lightness, or brightness in HSB); positions/readings come from
  `area_axes()` so HSB uses HSB saturation. Also `aria-keyshortcuts`,
  thumb-position custom properties, `tabindex` (`-1` when disabled) with
  `aria-disabled` when disabled, and `data-ars-dragging` **only** while the
  active drag target is the area (a hue/alpha slider drag does not flag it).
- **`channel_slider_attrs(channel)` / `channel_slider_thumb_attrs(channel)`** —
  `role="group"` / `role="slider"` with `data-ars-channel`, `aria-valuenow`/`min`/
  `max`, `aria-label`, the thumb-position custom property, and `data-ars-dragging`
  for the matching channel; `tabindex="-1"` + `aria-disabled` when disabled. The
  hue thumb reports the unwrapped `Context::hue_value` so it stays at the 360°
  endpoint instead of wrapping to 0°.
- **`swatch_attrs(index)`** — `type="button"`, `role="button"`, `data-ars-index`,
  `tabindex` (`-1` + `aria-disabled` when disabled), and (when `index` is in
  range) `aria-label` from `messages.swatch_label`, the `--ars-swatch-color`
  custom property, and `data-ars-selected` when the swatch equals the current
  value. An out-of-range index yields the base attributes only.
- **`channel_input_attrs(channel, index)`** — `type="text"`, `inputmode="numeric"`,
  `data-ars-channel`/`data-ars-channel-index`, the channel id, the current
  channel `value` (degrees for hue, raw `0`–`255` for RGB, integer percent for
  the fractional channels; space-aware via `channel_current` so HSB shows HSB
  saturation), and `disabled`/`readonly` mirrors.
- **`eye_dropper_trigger_attrs`** — `type="button"`, `aria-label`; `hidden` when
  `eyedropper_supported` is `false`; `disabled` when disabled or read-only.
- **`hidden_input_attrs`** — `type="hidden"`, optional `name`, and the canonical
  hex `value` (8-digit when `show_alpha` and translucent); `disabled` omits it
  from submission.

The `Api` also exposes typed event-dispatch helpers so adapters never hand-build
events: `on_trigger_click` / `on_trigger_keydown` (Enter/Space → `Toggle`),
`on_content_keydown` (Escape → `CloseOnEscape`), `on_area_pointer_down(x, y)` and
`on_channel_slider_pointer_down(channel, x)` (→ `DragStart`),
`on_area_thumb_keydown(data, shift)` (arrows → `AreaXStep`/`AreaYStep`,
RTL-mirrored on the x-axis; the machine applies them space-aware),
`on_channel_slider_keydown(channel, data, shift)` (arrows + Home/End),
`on_swatch_click(index)` (→ `SetColor`), and `on_eyedropper_click` (→
`EyedropperRequest`). Keyboard steps for the fractional channels (saturation,
lightness, brightness, alpha) come from `channel_step_default` so a single arrow
press is a perceptible 1%/10% nudge; the configured `channel_step` /
`channel_large_step` apply to the wider hue and RGB ranges.

```rust
impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Content => self.content_attrs(),
            Part::Area => self.area_attrs(),
            Part::AreaThumb => self.area_thumb_attrs(),
            Part::ChannelSlider { channel } => self.channel_slider_attrs(channel),
            Part::ChannelSliderThumb { channel } => self.channel_slider_thumb_attrs(channel),
            Part::AlphaSlider => self.alpha_slider_attrs(),
            Part::SwatchGroup => self.swatch_group_attrs(),
            Part::Swatch { index } => self.swatch_attrs(index),
            Part::FormatSelect => self.format_select_attrs(),
            Part::ChannelInput { channel, index } => self.channel_input_attrs(channel, index),
            Part::HexInput => self.hex_input_attrs(),
            Part::EyeDropperTrigger => self.eye_dropper_trigger_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
ColorPicker
├── Root                        (required)
├── Label                       (required — text label for the picker)
├── Control                     (required — container for trigger/swatch area)
├── Trigger                     (required — button to open/close popover)
├── Content                     (required — the popover panel)
│   ├── Area                    (required — 2D saturation/lightness gradient)
│   │   └── AreaThumb           (required — draggable thumb in the area)
│   ├── ChannelSlider[hue]      (required — hue channel strip)
│   │   └── ChannelSliderThumb  (required — draggable thumb in the channel slider)
│   ├── AlphaSlider             (optional — alpha channel strip, when show_alpha=true)
│   ├── ChannelInput x N        (optional — text inputs for individual channels)
│   ├── HexInput                (optional — hex color text input)
│   ├── FormatSelect            (optional — dropdown/button to choose format)
│   ├── SwatchGroup             (optional — preset color swatches)
│   │   └── Swatch x N          (optional — individual swatch buttons)
│   └── EyeDropperTrigger       (optional — browser eyedropper button)
└── HiddenInput                 (required — for form submission)
```

| Part                 | Element                 | Required | Key Attributes                                                                                                                                                          |
| -------------------- | ----------------------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Root`               | `<div>`                 | yes      | `data-ars-state`, `data-ars-disabled`, `data-ars-readonly`                                                                                                              |
| `Label`              | `<label>`               | yes      | `for` (trigger ID)                                                                                                                                                      |
| `Control`            | `<div>`                 | yes      |                                                                                                                                                                         |
| `Trigger`            | `<button>`              | yes      | `type="button"`, `aria-haspopup="dialog"`, `aria-expanded`, `aria-controls`, `aria-labelledby`                                                                          |
| `Content`            | `<div>`                 | yes      | `role="dialog"`, `aria-labelledby`, `data-ars-state`                                                                                                                    |
| `Area`               | `<div>`                 | yes      | `role="group"`                                                                                                                                                          |
| `AreaThumb`          | `<div>`                 | yes      | `role="application"`, `aria-roledescription`, `aria-valuetext` (saturation + lightness/brightness), `tabindex` (`0`, `-1` when disabled), `aria-disabled` when disabled |
| `ChannelSlider`      | `<div>`                 | yes      | `role="group"`, `data-ars-channel`                                                                                                                                      |
| `ChannelSliderThumb` | `<div>`                 | yes      | `role="slider"`, `tabindex` (`0`, `-1` when disabled), `aria-valuenow`, `aria-label`, `aria-disabled` when disabled                                                     |
| `AlphaSlider`        | `<div>`                 | no       | `role="group"`, `data-ars-channel="alpha"`                                                                                                                              |
| `SwatchGroup`        | `<div>`                 | no       | `role="group"`                                                                                                                                                          |
| `Swatch`             | `<button>`              | no       | `type="button"`, `role="button"`, `tabindex` (`0`, `-1` when disabled), `aria-label`, `data-ars-selected`, `data-ars-index`, `aria-disabled` when disabled              |
| `FormatSelect`       | `<select>` / `<button>` | no       | `aria-label`                                                                                                                                                            |
| `ChannelInput`       | `<input>`               | no       | `type="text"`, `inputmode="numeric"`, `value` (current channel value), `data-ars-channel`, `data-ars-channel-index`                                                     |
| `HexInput`           | `<input>`               | no       | `type="text"`, `inputmode="text"`                                                                                                                                       |
| `EyeDropperTrigger`  | `<button>`              | no       | `type="button"`, `aria-label`, `hidden` (when unsupported)                                                                                                              |
| `HiddenInput`        | `<input type="hidden">` | yes      | `name`, `value`                                                                                                                                                         |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Attribute / Behaviour               | Element                                                 | Value                                              |
| ----------------------------------- | ------------------------------------------------------- | -------------------------------------------------- |
| `role="dialog"`                     | `Content`                                               | Popover container                                  |
| `aria-haspopup="dialog"`            | `Trigger`                                               | Indicates popover                                  |
| `aria-expanded`                     | `Trigger`                                               | `"true"` / `"false"`                               |
| `aria-controls`                     | `Trigger`                                               | Content ID                                         |
| `aria-labelledby`                   | `Content`, `Trigger`                                    | Label ID                                           |
| `role="application"`                | `AreaThumb`                                             | 2D color area interaction                          |
| `aria-roledescription="color area"` | `AreaThumb`                                             | Describes the 2D area control                      |
| `role="slider"`                     | Channel slider thumbs                                   | Slider interaction                                 |
| `aria-valuenow`                     | Channel slider thumbs                                   | Current numeric value                              |
| `aria-valuemin` / `aria-valuemax`   | Channel slider thumbs                                   | Channel range                                      |
| `aria-valuetext`                    | `AreaThumb`                                             | Formatted color string                             |
| `aria-label`                        | `AreaThumb`                                             | `"Color area selector"`                            |
| `aria-label`                        | Channel slider thumbs                                   | `"Hue"`, `"Alpha"`                                 |
| `aria-label`                        | `EyeDropperTrigger`                                     | `"Pick color from screen"`                         |
| `aria-label`                        | `FormatSelect`                                          | `"Toggle color format"`                            |
| `aria-label`                        | `Swatch`                                                | `"Select color #rrggbb"`                           |
| `aria-live="polite"`                | Value text live region                                  | Announces color changes                            |
| `aria-disabled="true"`              | `Trigger`, `AreaThumb`, channel slider thumbs, `Swatch` | When disabled (these also drop to `tabindex="-1"`) |
| `aria-keyshortcuts`                 | `AreaThumb`                                             | Documents arrow key controls                       |

### 3.2 Keyboard Interaction

| Key         | Element                     | Action                      |
| ----------- | --------------------------- | --------------------------- |
| Enter/Space | `Trigger`                   | Toggle popover              |
| Enter/Space | `Swatch`                    | Select swatch color         |
| Escape      | `Content`                   | Close popover               |
| Arrow keys  | `AreaThumb`                 | Adjust saturation/lightness |
| Shift+Arrow | `AreaThumb`, channel slider | Large step adjustment       |
| Arrow keys  | Channel slider thumbs       | Adjust channel value        |
| Home/End    | Channel slider thumbs       | Jump to min/max             |
| Tab         | All interactive parts       | Standard focus navigation   |

### 3.3 Screen Reader Announcements

During keyboard-driven color adjustments, a debounced `aria-live="polite"` announcement (500ms after last keyboard interaction) reads the current color value in the active format (e.g., "Color: #ff3366"). The `Messages` struct includes `color_announcement: MessageFn<dyn Fn(&ColorValue, ColorFormat, &Locale) -> String + Send + Sync>` for this purpose.

When switching color spaces (via `Event::ChangeColorSpace`), an `aria-live="polite"` announcement reads `"Switched to {space} color space"` to inform screen reader users that the control layout has changed.

## 4. Internationalization

### 4.1 Messages

```rust
// `PartialEq` is derived (via `MessageFn`'s pointer-identity `PartialEq`) because
// `Context` embeds `Messages` and derives `PartialEq`.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    pub trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub area_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub area_role_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub hue_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub alpha_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub saturation_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub lightness_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub brightness_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub eyedropper_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub format_toggle_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub swatch_label: MessageFn<dyn Fn(&ColorValue, &Locale) -> String + Send + Sync>,
    pub color_name: MessageFn<dyn Fn(&ColorValue, &Locale) -> String + Send + Sync>,
    pub color_announcement: MessageFn<dyn Fn(&ColorValue, ColorFormat, &Locale) -> String + Send + Sync>,
    pub channel_value_text: MessageFn<dyn Fn(&str, &str, &str, &Locale) -> String + Send + Sync>,
    pub color_space_switched: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            trigger_label: MessageFn::static_str("Pick a color"),
            area_label: MessageFn::static_str("Color area selector"),
            area_role_description: MessageFn::static_str("color area"),
            hue_label: MessageFn::static_str("Hue"),
            alpha_label: MessageFn::static_str("Alpha"),
            saturation_label: MessageFn::static_str("Saturation"),
            lightness_label: MessageFn::static_str("Lightness"),
            brightness_label: MessageFn::static_str("Brightness"),
            eyedropper_label: MessageFn::static_str("Pick color from screen"),
            format_toggle_label: MessageFn::static_str("Toggle color format"),
            swatch_label: MessageFn::new(|color, _locale| format!("Select color {}", color.to_hex(false))),
            color_name: MessageFn::new(|color, _locale| color.color_name_en()),
            color_announcement: MessageFn::new(|color, format, _locale| {
                format!("Color: {}", format_color_string(color, *format))
            }),
            channel_value_text: MessageFn::new(|label, value, _unit, _locale| {
                format!("{}: {}", label, value)
            }),
            color_space_switched: MessageFn::new(|space, _locale| {
                format!("Switched to {} color space", space)
            }),
        }
    }
}

impl ComponentMessages for Messages {}
```

| Key                                | Default (en-US)            | Purpose                         |
| ---------------------------------- | -------------------------- | ------------------------------- |
| `color_picker.label`               | `"Color picker"`           | Default label text              |
| `color_picker.trigger_label`       | `"Pick a color"`           | Trigger aria-label              |
| `color_picker.area_label`          | `"Color area selector"`    | Area thumb aria-label           |
| `color_picker.hue_label`           | `"Hue"`                    | Hue slider label                |
| `color_picker.alpha_label`         | `"Alpha"`                  | Alpha slider label              |
| `color_picker.eyedropper_label`    | `"Pick color from screen"` | Eyedropper button label         |
| `color_picker.format_toggle_label` | `"Toggle color format"`    | Format button label             |
| `color_picker.swatch_label`        | `"Select color {color}"`   | Swatch button label (templated) |

- **RTL**: Channel sliders reverse direction; the area gradient flips horizontally so
  that right-to-left reading order places higher saturation on the leading edge.
- **Number formatting**: Channel input values respect locale decimal separators when
  formatting display values (e.g., `"0,75"` vs `"0.75"`).

## 5. Variant: Eyedropper

The EyeDropper API integration is a browser-dependent variant that adds screen color sampling capability.

### 5.1 Behavior

- The `eyedropper_supported` context field starts `false`. When the picker opens, the machine emits the `Effect::DetectEyedropper` intent; the adapter performs the `"EyeDropper" in window` runtime detection (the core never touches `window`) and reports the result back via `Event::SetEyedropperSupported(bool)`.
- When `eyedropper_supported` is `false`, the `EyeDropperTrigger` part is hidden (`hidden` attribute set).
- `Event::EyedropperRequest` is only processed when `eyedropper_supported` is `true` and the component is not `readonly`.
- The adapter calls `EyeDropper.open()` directly from a click event handler (within user gesture, not from an async callback) to satisfy the transient activation requirement.
- On success, `Event::EyedropperResult(Some(color))` updates the current value. On cancellation, `Event::EyedropperResult(None)` is sent and no change occurs.

### 5.2 Anatomy Additions

The `EyeDropperTrigger` part is conditionally rendered based on browser support. See the `Part::EyeDropperTrigger` variant in the Part enum.

### 5.3 Accessibility

The eyedropper trigger button uses `aria-label` from `messages.eyedropper_label` (default: "Pick color from screen"). When the API is unavailable, the button is hidden from both visual and accessibility trees via the `hidden` attribute.

**Browser support:** The EyeDropper API is Chromium-only (Chrome 95+, Edge 95+). Not available in Firefox or Safari.

## 6. Library Parity

> Compared against: Ark UI (`ColorPicker`), React Aria (`ColorPicker`).

### 6.1 Props

| Feature                    | ars-ui                    | Ark UI                     | React Aria                     | Notes                                                                                   |
| -------------------------- | ------------------------- | -------------------------- | ------------------------------ | --------------------------------------------------------------------------------------- |
| `value` / `defaultValue`   | `value` / `default_value` | `value` / `defaultValue`   | `value` / `defaultValue`       | Equivalent                                                                              |
| `format` / `defaultFormat` | `default_format`          | `format` / `defaultFormat` | --                             | Ark has controlled format; ars-ui uses `Event::SetFormat`                               |
| `open` / `defaultOpen`     | `open` / `default_open`   | `open` / `defaultOpen`     | --                             | Equivalent                                                                              |
| `disabled`                 | `disabled`                | `disabled`                 | --                             | Equivalent                                                                              |
| `readOnly`                 | `readonly`                | `readOnly`                 | --                             | Equivalent                                                                              |
| `invalid`                  | --                        | `invalid`                  | --                             | Ark-only; ars-ui validates at form level                                                |
| `required`                 | --                        | `required`                 | --                             | Ark-only; ars-ui validates at form level                                                |
| `closeOnSelect`            | --                        | `closeOnSelect`            | --                             | Ark-only; ars-ui leaves swatch selection behavior to adapter                            |
| `inline`                   | --                        | `inline`                   | --                             | Ark-only; ars-ui renders via open state                                                 |
| `name`                     | `name`                    | `name`                     | --                             | Equivalent                                                                              |
| `positioning`              | `positioning`             | `positioning`              | --                             | Equivalent                                                                              |
| `colorSpace`               | `color_space`             | --                         | --                             | ars-ui exclusive                                                                        |
| `showAlpha`                | `show_alpha`              | --                         | --                             | ars-ui exclusive                                                                        |
| `swatches`                 | `swatches`                | (SwatchGroup children)     | (separate `ColorSwatchPicker`) | ars-ui exposes presets as a `Vec<ColorValue>` prop resolved by `Part::Swatch { index }` |
| `on_change_end`            | `on_change_end`           | `onValueChangeEnd`         | `onChange`                     | Equivalent intent                                                                       |

**Gaps:** None worth adopting. `invalid`/`required` are form-level concerns. `closeOnSelect`/`inline` are minor UX preferences best handled in the adapter.

### 6.2 Anatomy

| Part               | ars-ui               | Ark UI                           | React Aria                     | Notes                                              |
| ------------------ | -------------------- | -------------------------------- | ------------------------------ | -------------------------------------------------- |
| Root               | `Root`               | `Root`                           | `ColorPicker`                  | Equivalent                                         |
| Label              | `Label`              | `Label`                          | --                             | Equivalent                                         |
| Control            | `Control`            | `Control`                        | --                             | Equivalent                                         |
| Trigger            | `Trigger`            | `Trigger`                        | --                             | Equivalent                                         |
| Content            | `Content`            | `Content`                        | --                             | Equivalent                                         |
| Area               | `Area`               | `Area`                           | (separate `ColorArea`)         | Equivalent                                         |
| AreaThumb          | `AreaThumb`          | `AreaThumb`                      | `ColorThumb`                   | Equivalent                                         |
| ChannelSlider      | `ChannelSlider`      | `ChannelSlider`                  | (separate `ColorSlider`)       | Equivalent                                         |
| ChannelSliderThumb | `ChannelSliderThumb` | `ChannelSliderThumb`             | `ColorThumb`                   | Equivalent                                         |
| AlphaSlider        | `AlphaSlider`        | (via ChannelSlider)              | --                             | ars-ui has dedicated alpha part                    |
| SwatchGroup        | `SwatchGroup`        | `SwatchGroup`                    | (separate `ColorSwatchPicker`) | Equivalent                                         |
| Swatch             | `Swatch`             | `Swatch`                         | `ColorSwatch`                  | Equivalent                                         |
| FormatSelect       | `FormatSelect`       | `FormatSelect` / `FormatTrigger` | --                             | Equivalent                                         |
| ChannelInput       | `ChannelInput`       | `ChannelInput`                   | --                             | Equivalent                                         |
| HexInput           | `HexInput`           | --                               | --                             | ars-ui exclusive                                   |
| EyeDropperTrigger  | `EyeDropperTrigger`  | `EyeDropperTrigger`              | --                             | Equivalent                                         |
| HiddenInput        | `HiddenInput`        | `HiddenInput`                    | --                             | Equivalent                                         |
| TransparencyGrid   | --                   | `TransparencyGrid`               | --                             | Ark-only; adapter CSS concern                      |
| ValueText          | --                   | `ValueText`                      | --                             | Ark-only; covered by `value_as_string()` on Api    |
| ValueSwatch        | --                   | `ValueSwatch`                    | --                             | Ark-only; adapter can render via trigger           |
| View               | --                   | `View`                           | --                             | Ark-only; format-specific views handled by adapter |

**Gaps:** None. `TransparencyGrid`, `ValueText`, `ValueSwatch`, and `View` are rendering/layout concerns handled by adapters, not core anatomy.

### 6.3 Events

| Callback         | ars-ui                      | Ark UI             | React Aria | Notes                  |
| ---------------- | --------------------------- | ------------------ | ---------- | ---------------------- |
| Value change     | `Bindable` reactivity       | `onValueChange`    | `onChange` | Equivalent via binding |
| Value change end | `on_change_end`             | `onValueChangeEnd` | --         | Equivalent             |
| Open change      | `Bindable<bool>` reactivity | `onOpenChange`     | --         | Equivalent via binding |
| Format change    | `Event::SetFormat`          | `onFormatChange`   | --         | Equivalent             |

**Gaps:** None.

### 6.4 Features

| Feature               | ars-ui                 | Ark UI         | React Aria           |
| --------------------- | ---------------------- | -------------- | -------------------- |
| Multiple color spaces | Yes (HSL/HSB/RGB/HEX)  | Yes            | Yes                  |
| Color space switching | Yes (runtime)          | Yes (via View) | --                   |
| Eyedropper API        | Yes                    | Yes            | --                   |
| Swatch presets        | Yes                    | Yes            | --                   |
| Alpha channel         | Yes (`show_alpha`)     | Yes            | --                   |
| Keyboard interaction  | Yes (full)             | Yes            | Yes                  |
| RTL support           | Yes                    | Yes            | --                   |
| Color name (a11y)     | Yes (`ColorNameParts`) | --             | Yes (`getColorName`) |
| Contrast ratio utils  | Yes (`contrast_ratio`) | --             | --                   |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity.
- **Divergences:** Ark UI uses a monolithic ColorPicker with sub-parts; ars-ui separates ColorArea, ColorSlider, ColorWheel, ColorField, ColorSwatch, and ColorSwatchPicker as independent components that can also be composed inside ColorPicker. React Aria follows the same separated approach.
- **Recommended additions:** None.
