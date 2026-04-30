---
component: ColorPicker
category: specialized
tier: complex
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related:
    [
        color-area,
        color-slider,
        color-field,
        color-swatch,
        color-swatch-picker,
        color-wheel,
        angle-slider,
    ]
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

```rust
// crates/ars-core/src/components/color_picker.rs

use crate::{Bindable, ComponentId};
use crate::machine::{Machine, TransitionPlan, ComponentIds, AttrMap};

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
    pub fn new(hue: f64, saturation: f64, lightness: f64, alpha: f64) -> Self {
        debug_assert!(hue.is_finite(), "hue must be finite");
        debug_assert!(saturation.is_finite(), "saturation must be finite");
        debug_assert!(lightness.is_finite(), "lightness must be finite");
        debug_assert!(alpha.is_finite(), "alpha must be finite");
        Self {
            hue: hue.rem_euclid(360.0),
            saturation: saturation.clamp(0.0, 1.0),
            lightness: lightness.clamp(0.0, 1.0),
            alpha: alpha.clamp(0.0, 1.0),
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
        let hex = hex.trim_start_matches('#');
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
    /// Returns localized color description parts for accessibility.
    /// Parts include lightness ("light"/"dark"), saturation ("vivid"/"muted"),
    /// and hue name ("blue", "red", etc.) in the given locale.
    ///
    /// Algorithm:
    /// 1. Convert HSL -> sRGB -> OKLCH (perceptually uniform lightness).
    /// 2. Map OKLCH lightness to 5 levels: very dark (0-0.2), dark (0.2-0.4),
    ///    medium/omitted (0.4-0.6), light (0.6-0.8), very light (0.8-1.0).
    /// 3. Map OKLCH chroma: grayish (0-0.04), moderate/omitted (0.04-0.12),
    ///    vibrant (>0.12). If lightness > 0.7 and chroma moderate -> "pale".
    /// 4. Map OKLCH hue angle to 13 named hues: red, red-orange, orange,
    ///    yellow-orange, yellow, yellow-green, green, cyan-green, cyan,
    ///    cyan-blue, blue, purple, magenta.
    /// 5. Special-case: near-zero chroma -> "gray"/"white"/"black" by lightness.
    ///
    /// Returns parts so i18n can reorder (e.g., English: "{lightness} {chroma}
    /// {hue}", Italian: "{hue} {chroma} {lightness}").
    /// Delegate to locale-aware color naming from Messages/CLDR.
    pub fn color_name_parts(&self, locale: &Locale, messages: &Messages) -> ColorNameParts {
        // OKLCH classification to determine lightness_level, saturation_level, hue_bucket
        // then delegate to messages for localized strings
        let (lightness_level, saturation_level, hue_bucket) = self.oklch_classify();
        ColorNameParts {
            lightness: (messages.lightness_label)(lightness_level),
            chroma: (messages.saturation_label)(saturation_level),
            hue: (messages.hue_name)(hue_bucket),
        }
    }

    /// Convenience: format as English-order string "dark vibrant blue".
    /// Joins non-empty parts with spaces.
    pub fn color_name_en(&self) -> String {
        // Fallback English-only path for tests and non-localized contexts
        let parts = self.color_name_parts_en();
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
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

/// Parse "h, s%, b%" inside hsb().
fn parse_hsb_args(inner: &str) -> Option<ColorValue> {
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() != 3 { return None; }
    let h: f64 = parts[0].parse().ok()?;
    let s: f64 = parts[1].strip_suffix('%')?.trim().parse::<f64>().ok()? / 100.0;
    let b: f64 = parts[2].strip_suffix('%')?.trim().parse::<f64>().ok()? / 100.0;
    // Convert HSB -> HSL: lightness = b * (1 - s/2)
    let l = b * (1.0 - s / 2.0);
    let sl = if l > 0.0 && l < 1.0 { (b - l) / l.min(1.0 - l) } else { 0.0 };

    Some(ColorValue::new(h, sl, l, 1.0))
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
    if let Some(inner) = strip_fn_call(trimmed, "hsb") {
        return parse_hsb_args(inner);
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
            format!("hsb({:.0}, {:.1}%, {:.1}%)", h, s * 100.0, b * 100.0)
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
    /// Close on interact outside.
    CloseOnInteractOutside,
    /// Close on escape.
    CloseOnEscape,
}
```

### 1.3 Context

```rust
/// The context for the `ColorPicker` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The current color value (controlled or uncontrolled).
    pub value: Bindable<ColorValue>,
    /// The currently displayed format in the input area.
    pub format: ColorFormat,
    /// Whether the picker popover is open.
    pub open: Bindable<bool>,
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
    /// Component instance IDs.
    pub id: ComponentId,
    /// The id of the trigger element.
    pub trigger_id: String,
    /// The id of the content element.
    pub content_id: String,
    /// The id of the area element.
    pub area_id: String,
    /// The id of the area thumb element.
    pub area_thumb_id: String,
    /// The id of the hue slider element.
    pub hue_slider_id: String,
    /// The id of the alpha slider element.
    pub alpha_slider_id: String,
    /// The id of the swatch trigger element.
    pub swatch_trigger_id: String,
    /// The id of the label element.
    pub label_id: String,
    /// The id of the format select element.
    pub format_select_id: String,
    /// The ids of the channel input elements.
    pub channel_input_ids: [String; 4], // R/H, G/S, B/L, A
    /// Text direction for RTL-aware keyboard navigation.
    pub dir: Direction,
    /// Active color space for the picker controls.
    pub color_space: ColorSpace,
    /// Locale for internationalized messages.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
}

impl Context {
    /// Returns the channels available in the current color space.
    pub fn channels(&self) -> &[ColorChannel] {
        match self.color_space {
            ColorSpace::Rgb => &[ColorChannel::Red, ColorChannel::Green, ColorChannel::Blue],
            ColorSpace::Hsl => &[ColorChannel::Hue, ColorChannel::Saturation, ColorChannel::Lightness],
            ColorSpace::Hsb => &[ColorChannel::Hue, ColorChannel::Saturation, ColorChannel::Brightness],
            ColorSpace::Hwb => &[ColorChannel::Hue, ColorChannel::Saturation, ColorChannel::Lightness], // HWB mapped
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
    /// Text direction for RTL-aware keyboard navigation and layout.
    /// Default: `Direction::Ltr`.
    pub dir: Direction,
    /// Name attribute for the hidden form input.
    pub name: Option<String>,
    /// Component instance ID.
    pub id: String,
    /// Callback fired when a drag interaction ends (pointerup on area/slider/wheel).
    /// Unlike continuous change callbacks, this fires once at the end of the gesture.
    /// Use for expensive operations like saving to a server.
    pub on_change_end: Option<Callback<ColorValue>>,
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
    *ctx.open.get()
}
```

### 1.6 Color Space Switching

`ColorPicker` supports runtime color space switching via `Event::ChangeColorSpace(ColorSpace)`.

**Transition Behavior** for `ChangeColorSpace(new_space)`:

1. **Recompute value**: Convert the current color value from the old color space to `new_space`. Preserve perceptual magnitudes where possible (e.g., HSL->HSB preserves hue and saturation, converts lightness to brightness).
2. **Update context**: Set `ctx.color_space = new_space`. Update `ctx.value` with the converted color.
3. **Remap channels**: The color area and channel sliders update to reflect the new space's channels (e.g., switching from RGB to HSL changes the area from Red/Green to Hue/Saturation, and the slider from Blue to Lightness).
4. **Live region announcement**: Emit an `aria-live="polite"` announcement: `"Switched to {space} color space"` (e.g., "Switched to HSL color space"). This informs screen reader users that the control layout has changed.

**UI Affordance**: The color space selector (button group, dropdown, or segmented control) emits `Event::ChangeColorSpace(selected_space)` when the user selects a different space. The spec does not prescribe a specific UI pattern -- adapters choose the appropriate control.

**Supported Spaces**: All spaces in the `ColorSpace` enum (RGB, HSL, HSB/HSV, HEX) are switchable at runtime. The conversion between spaces is lossless for the sRGB gamut (minor floating-point rounding is acceptable).

### 1.7 Full Machine Implementation

```rust
/// Apply a pointer position to the color value for the given drag target.
/// For Area: x maps to saturation, y maps to lightness (inverted: top=light).
/// For Channel: x maps to the channel's full range via `with_channel()`.
fn apply_drag_position(ctx: &mut Context, target: DragTarget, x: f64, y: f64) {
    let current = ctx.value.get().clone();
    match target {
        DragTarget::Area => {
            // x maps to saturation [0.0, 1.0], y maps to lightness [1.0, 0.0]
            let s = x.clamp(0.0, 1.0);
            let l = (1.0 - y).clamp(0.0, 1.0);
            ctx.value.set(ColorValue::new(current.hue, s, l, current.alpha));
        }
        DragTarget::Channel(ch) => {
            let (min, max) = channel_range(ch);
            let value = min + x.clamp(0.0, 1.0) * (max - min);
            ctx.value.set(with_channel(&current, ch, value));
        }
    }
}

/// The machine for the `ColorPicker` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let value = match &props.value {
            Some(v) => Bindable::controlled(v.clone()),
            None => Bindable::uncontrolled(props.default_value.clone()),
        };

        let open = match props.open {
            Some(v) => Bindable::controlled(v),
            None => Bindable::uncontrolled(props.default_open),
        };

        let state = if *open.get() {
            State::Open
        } else {
            State::Closed
        };

        let ids = ComponentIds::from_id(&props.id);
        let locale = env.locale.clone();
        let messages = messages.clone();

        (state, Context {
            value,
            format: props.default_format,
            open,
            disabled: props.disabled,
            readonly: props.readonly,
            close_on_interact_outside: props.close_on_interact_outside,
            close_on_escape: props.close_on_escape,
            show_alpha: props.show_alpha,
            eyedropper_supported: false, // Detected at runtime via Effect
            focused_part: None,
            positioning: props.positioning.clone(),
            channel_step: props.channel_step,
            channel_large_step: props.channel_large_step,
            id: ids.id().to_string().into(),
            trigger_id: ids.part("trigger"),
            content_id: ids.part("content"),
            area_id: ids.part("area"),
            area_thumb_id: ids.part("area-thumb"),
            hue_slider_id: ids.part("hue-slider"),
            alpha_slider_id: ids.part("alpha-slider"),
            swatch_trigger_id: ids.part("swatch-trigger"),
            label_id: ids.part("label"),
            format_select_id: ids.part("format-select"),
            channel_input_ids: [
                ids.part("channel-0"),
                ids.part("channel-1"),
                ids.part("channel-2"),
                ids.part("channel-3"),
            ],
            dir: props.dir,
            color_space: props.color_space,
            locale,
            messages,
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // Global guard: disabled components ignore all events except Focus/Blur.
        if ctx.disabled {
            return match event {
                Event::Focus { part } => {
                    let part = *part;
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.focused_part = Some(part);
                    }))
                }
                Event::Blur { .. } => {
                    Some(TransitionPlan::context_only(|ctx| {
                        ctx.focused_part = None;
                    }))
                }
                _ => None,
            };
        }

        match (state, event) {
            // --- Closed state ---
            (State::Closed, Event::Open)
            | (State::Closed, Event::Toggle) => {
                Some(TransitionPlan::to(State::Open).apply(|ctx| {
                    ctx.open.set(true);
                }).with_named_effect("click-outside", |ctx, _props, send| {
                    let content_id = ctx.content_id.clone();
                    let trigger_id = ctx.trigger_id.clone();
                    let cleanup = add_click_outside_listener_multi(
                        &[&content_id, &trigger_id],
                        move || {
                            send(Event::CloseOnInteractOutside);
                        },
                    );
                    cleanup
                }).with_named_effect("detect-eyedropper", |ctx, _props, _send| {
                    // Check if EyeDropper API is available
                    let supported = is_eyedropper_available();
                    // Store result -- in practice this would set context
                    no_cleanup()
                }))
            }

            // --- Open state ---
            (State::Open, Event::Close)
            | (State::Open, Event::Toggle) => {
                Some(TransitionPlan::to(State::Closed).apply(|ctx| {
                    ctx.open.set(false);
                }))
            }

            (State::Open, Event::CloseOnInteractOutside) => {
                if ctx.close_on_interact_outside {
                    Some(TransitionPlan::to(State::Closed).apply(|ctx| {
                        ctx.open.set(false);
                    }))
                } else {
                    None
                }
            }

            // When dragging, InteractOutside is suppressed -- the user is still
            // interacting with the picker via pointer capture
            (State::Dragging { .. }, Event::CloseOnInteractOutside) => {
                None // Suppress during drag
            }

            (State::Open, Event::CloseOnEscape) => {
                if ctx.close_on_escape {
                    Some(TransitionPlan::to(State::Closed).apply(|ctx| {
                        ctx.open.set(false);
                    }))
                } else {
                    None
                }
            }

            (State::Open, Event::DragStart { target, x, y }) => {
                if ctx.readonly { return None; }
                let target = *target;
                let x = *x;
                let y = *y;
                Some(TransitionPlan::to(State::Dragging { target }).apply(move |ctx| {
                    apply_drag_position(ctx, target, x, y);
                }).with_named_effect("drag-listeners", move |_ctx, _props, send| {
                    let platform = use_platform_effects();
                    let send_move = send.clone();
                    let send_up = send.clone();
                    platform.track_pointer_drag(
                        Box::new(move |x, y| { send_move.call_if_alive(Event::DragMove { x, y }); }),
                        Box::new(move || { send_up.call_if_alive(Event::DragEnd); }),
                    )
                }))
            }

            // --- Dragging state ---
            (State::Dragging { target }, Event::DragMove { x, y }) => {
                let target = *target;

                let x = *x;
                let y = *y;

                Some(TransitionPlan::context_only(move |ctx| {
                    apply_drag_position(ctx, target, x, y);
                }))
            }

            (State::Dragging { .. }, Event::DragEnd) => {
                let final_color = ctx.value.get().clone();
                Some(TransitionPlan::to(State::Open)
                    .with_effect(PendingEffect::new("on-change-end", move |_ctx, props, _send| {
                        if let Some(ref cb) = props.on_change_end {
                            cb.call(final_color);
                        }
                        no_cleanup()
                    })))
            }

            // --- Events valid in any state (including Closed/Idle) ---
            (State::Closed, Event::SetColor(color))
            | (State::Open, Event::SetColor(color))
            | (State::Dragging { .. }, Event::SetColor(color)) => {
                if ctx.readonly { return None; }

                let color = color.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(color);
                }))
            }

            (State::Open, Event::SetChannel { channel, value }) => {
                if ctx.readonly { return None; }

                let channel = *channel;

                let value = *value;

                Some(TransitionPlan::context_only(move |ctx| {
                    let color = ctx.value.get();
                    ctx.value.set(with_channel(color, channel, value));
                }))
            }

            (_, Event::SetFormat(format)) => {
                let format = *format;

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.format = format;
                }))
            }

            (_, Event::ChangeColorSpace(new_space)) => {
                let new_space = *new_space;

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.color_space = new_space;
                }).with_named_effect("announce-color-space", move |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    let text = (ctx.messages.color_space_switched)(
                        &format!("{:?}", new_space), &ctx.locale);
                    platform.announce(&text);
                    no_cleanup()
                }))
            }

            (State::Open, Event::ChannelIncrement { channel, step }) => {
                if ctx.readonly { return None; }

                let channel = *channel;

                let step = *step;

                Some(TransitionPlan::context_only(move |ctx| {
                    let color = ctx.value.get();
                    let current = channel_value(color, channel);
                    let (_, max) = channel_range(channel);
                    let next = (current + step).min(max);
                    ctx.value.set(with_channel(color, channel, next));
                }))
            }

            (State::Open, Event::ChannelDecrement { channel, step }) => {
                if ctx.readonly { return None; }

                let channel = *channel;

                let step = *step;

                Some(TransitionPlan::context_only(move |ctx| {
                    let color = ctx.value.get();
                    let current = channel_value(color, channel);
                    let (min, _) = channel_range(channel);
                    let next = (current - step).max(min);
                    ctx.value.set(with_channel(color, channel, next));
                }))
            }

            (State::Open, Event::EyedropperRequest) => {
                if !ctx.eyedropper_supported || ctx.readonly { return None; }

                Some(TransitionPlan::context_only(|_ctx| {
                }).with_named_effect("eyedropper", |_ctx, _props, send| {
                    let cleanup = invoke_eyedropper(move |result| {
                        send(Event::EyedropperResult(result));
                    });
                    cleanup
                }))
            }

            (_, Event::EyedropperResult(Some(color))) => {
                let color = color.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(color);
                }))
            }

            (_, Event::Focus { part }) => {
                let part = *part;

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_part = Some(part);
                }))
            }

            (_, Event::Blur { .. }) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused_part = None;
                }))
            }

            _ => None,
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
    ChannelSlider { channel: ColorChannel },
    ChannelSliderThumb { channel: ColorChannel },
    AlphaSlider,
    SwatchGroup,
    Swatch { index: usize, color: ColorValue },
    FormatSelect,
    ChannelInput { channel: ColorChannel, index: usize },
    HexInput,
    EyeDropperTrigger,
    HiddenInput,
}

/// The connect API for the `ColorPicker` component.
pub struct Api<'a> {
    /// The current state of the component.
    state: &'a State,
    /// The context of the component.
    ctx: &'a Context,
    /// The props of the component.
    props: &'a Props,
    /// The send function to send events to the component.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    // --- Computed state ---

    /// Whether the component is open.
    pub fn is_open(&self) -> bool {
        !matches!(self.state, State::Closed)
    }

    /// Whether the component is dragging.
    pub fn is_dragging(&self) -> bool {
        matches!(self.state, State::Dragging { .. })
    }

    /// The current value of the component.
    pub fn value(&self) -> &ColorValue {
        self.ctx.value.get()
    }

    /// The current value of the component as a string.
    pub fn value_as_string(&self) -> String {
        match self.ctx.format {
            ColorFormat::Hex => self.ctx.value.get().to_hex(self.ctx.show_alpha),
            ColorFormat::Hsl => self.ctx.value.get().to_css_hsl(),
            ColorFormat::Rgb => {
                let (r, g, b) = self.ctx.value.get().to_rgb();
                if self.ctx.show_alpha && self.ctx.value.get().alpha < 1.0 {
                    format!("rgba({}, {}, {}, {:.2})", r, g, b, self.ctx.value.get().alpha)
                } else {
                    format!("rgb({}, {}, {})", r, g, b)
                }
            }
            ColorFormat::Hsb => {
                let (h, s, b) = self.ctx.value.get().to_hsb();
                format!("hsb({:.0}, {:.1}%, {:.1}%)", h, s * 100.0, b * 100.0)
            }
        }
    }

    /// The current format of the component.
    pub fn format(&self) -> ColorFormat {
        self.ctx.format
    }

    // --- Imperative actions ---

    /// Open the component.
    pub fn open(&self) {
        (self.send)(Event::Open);
    }

    /// Close the component.
    pub fn close(&self) {
        (self.send)(Event::Close);
    }

    /// Set the value of the component.
    pub fn set_value(&self, color: ColorValue) {
        (self.send)(Event::SetColor(color));
    }

    /// Set the format of the component.
    pub fn set_format(&self, format: ColorFormat) {
        (self.send)(Event::SetFormat(format));
    }

    // --- Part attrs ---

    /// The attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }
        attrs
    }

    /// The attributes for the label element.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.label_id);
        attrs.set(HtmlAttr::For, &self.ctx.trigger_id);
        attrs
    }

    /// The attributes for the control element.
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Returns a human-readable name for the current color (e.g., "dark vibrant blue").
    /// Used as an accessible description or display text.
    pub fn color_name(&self) -> String {
        (self.ctx.messages.color_name)(self.ctx.value.get(), &self.ctx.locale)
    }

    /// Returns the announcement text for the current color in the given format.
    /// The adapter uses this for debounced `aria-live="polite"` announcements
    /// during keyboard-driven color adjustments (see §3.3).
    pub fn color_announcement(&self) -> String {
        (self.ctx.messages.color_announcement)(self.ctx.value.get(), self.ctx.format, &self.ctx.locale)
    }

    /// The attributes for the trigger element.
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.trigger_id);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.trigger_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.is_open() { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), &self.ctx.content_id);
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), &self.ctx.label_id);
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        // Event handlers (click/keydown to toggle) are typed methods on the Api struct.
        attrs
    }

    /// The attributes for the content element.
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.content_id);
        attrs.set(HtmlAttr::Role, "dialog");
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), &self.ctx.label_id);
        // Event handlers (keydown for Escape) are typed methods on the Api struct.
        attrs
    }

    /// The attributes for the area element.
    pub fn area_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Area.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.area_id);
        attrs.set(HtmlAttr::Role, "group");
        // Background gradient is the hue at full saturation
        let color = self.ctx.value.get();
        let bg = format!("hsl({:.0}, 100%, 50%)", color.hue);
        attrs.set_style(CssProperty::Custom("ars-color-picker-area-bg"), bg);
        // Event handlers (pointerdown to start drag) are typed methods on the Api struct.
        attrs
    }

    /// 2D Color Area keyboard navigation:
    /// - Left/Right -> change saturation (default step 1%, Shift step 10%)
    /// - Up/Down -> change lightness (default step 1%, Shift step 10%)
    /// Uses `role="application"` because `role="slider"` is 1D only.
    // Note: JAWS treats role="application" as a pass-through and does not synthesize
    // keyboard patterns. JAWS users rely on aria-roledescription="color area" and
    // aria-valuetext for orientation. Consider adding aria-keyshortcuts to document
    // available keyboard controls (Arrow keys for saturation/lightness adjustment).
    pub fn area_thumb_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::AreaThumb.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.area_thumb_id);
        attrs.set(HtmlAttr::Role, "application");
        attrs.set(HtmlAttr::Aria(AriaAttr::RoleDescription), (self.ctx.messages.area_role_description)(&self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.area_label)(&self.ctx.locale));
        let color = self.ctx.value.get();
        let sat_text = (self.ctx.messages.channel_value_text)(
            (self.ctx.messages.saturation_label)(&self.ctx.locale),
            &format!("{:.0}%", (color.saturation * 100.0).round()),
            "",
            &self.ctx.locale,
        );
        let light_text = (self.ctx.messages.channel_value_text)(
            (self.ctx.messages.lightness_label)(&self.ctx.locale),
            &format!("{:.0}%", (color.lightness * 100.0).round()),
            "",
            &self.ctx.locale,
        );
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), format!("{}, {}", sat_text, light_text));
        if self.is_dragging() {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }
        let color = self.ctx.value.get();
        attrs.set_style(CssProperty::Custom("ars-color-picker-area-thumb-x"), format!("{}%", color.saturation * 100.0));
        attrs.set_style(CssProperty::Custom("ars-color-picker-area-thumb-y"), format!("{}%", (1.0 - color.lightness) * 100.0));
        attrs.set_style(CssProperty::BackgroundColor, color.to_css_hsl());
        attrs.set(HtmlAttr::Aria(AriaAttr::KeyShortcuts), "ArrowUp ArrowDown ArrowLeft ArrowRight");
        // Event handlers (keydown for arrow keys, focus, blur) are typed methods on the Api struct.
        attrs
    }

    /// Attributes for a channel slider container. `channel` identifies which
    /// channel this slider controls (e.g., `ColorChannel::Hue`).
    pub fn channel_slider_attrs(&self, channel: ColorChannel) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ChannelSlider { channel }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let slider_id = match channel {
            ColorChannel::Hue => &self.ctx.hue_slider_id,
            ColorChannel::Alpha => &self.ctx.alpha_slider_id,
            _ => &self.ctx.hue_slider_id, // fallback
        };
        attrs.set(HtmlAttr::Id, slider_id);
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Data("ars-channel"), match channel {
            ColorChannel::Hue => "hue",
            ColorChannel::Alpha => "alpha",
            _ => "hue",
        });
        attrs
    }

    /// Attributes for the channel slider thumb (the draggable handle).
    pub fn channel_slider_thumb_attrs(&self, channel: ColorChannel) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ChannelSliderThumb { channel }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "slider");
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs.set(HtmlAttr::Data("ars-channel"), match channel {
            ColorChannel::Hue => "hue",
            ColorChannel::Alpha => "alpha",
            _ => "hue",
        });
        let color = self.ctx.value.get();
        let value = channel_value(color, channel);
        let (min, max) = channel_range(channel);
        let label = match channel {
            ColorChannel::Hue => (self.ctx.messages.hue_label)(&self.ctx.locale),
            ColorChannel::Alpha => (self.ctx.messages.alpha_label)(&self.ctx.locale),
            _ => (self.ctx.messages.hue_label)(&self.ctx.locale),
        };
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), format!("{:.0}", value));
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), format!("{:.0}", min));
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), format!("{:.0}", max));
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), "horizontal");
        let pct = if (max - min).abs() > f64::EPSILON { (value - min) / (max - min) * 100.0 } else { 0.0 };
        attrs.set_style(CssProperty::Custom("ars-color-picker-channel-thumb-position"), format!("{:.1}%", pct));
        if matches!(self.state, State::Dragging { target: DragTarget::Channel(c) } if *c == channel) {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }
        // Event handlers (keydown for arrow/Home/End channel adjustment) are typed methods on the Api struct.
        attrs
    }

    /// Attributes for the alpha slider container.
    pub fn alpha_slider_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::AlphaSlider.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.alpha_slider_id);
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Data("ars-channel"), "alpha");
        attrs
    }

    /// The attributes for the swatch group element.
    pub fn swatch_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::SwatchGroup.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "group");
        attrs
    }

    /// The attributes for a swatch trigger element.
    pub fn swatch_attrs(&self, index: usize, color: &ColorValue) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Swatch { index, color: *color }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "button");
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.swatch_label)(color, &self.ctx.locale));
        let is_selected = self.ctx.value.get() == color;
        if is_selected {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        }
        attrs.set_style(CssProperty::Custom("ars-swatch-color"), color.to_css_hsl());
        attrs.set(HtmlAttr::Data("ars-index"), index.to_string());
        // Event handlers (click to set swatch color) are typed methods on the Api struct.
        attrs
    }

    /// The attributes for the format select element.
    pub fn format_select_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::FormatSelect.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.format_select_id);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.format_toggle_label)(&self.ctx.locale));
        // Event handlers (change to cycle format) are typed methods on the Api struct.
        attrs
    }

    /// The attributes for a channel input element.
    pub fn channel_input_attrs(&self, channel: ColorChannel, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ChannelInput { channel, index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let id = &self.ctx.channel_input_ids[index.min(3)];
        attrs.set(HtmlAttr::Id, id);
        attrs.set(HtmlAttr::Type, "text");
        attrs.set(HtmlAttr::InputMode, "numeric");
        attrs.set(HtmlAttr::Data("ars-channel-index"), index.to_string());
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::ReadOnly, true);
        }
        attrs
    }

    /// The attributes for the hex input element.
    pub fn hex_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HexInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "text");
        attrs.set(HtmlAttr::InputMode, "text");
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::ReadOnly, true);
        }
        attrs
    }

    /// The attributes for the eye dropper trigger element.
    pub fn eye_dropper_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::EyeDropperTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.eyedropper_label)(&self.ctx.locale));
        if !self.ctx.eyedropper_supported {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        if self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        // Event handlers (click to request eyedropper) are typed methods on the Api struct.
        attrs
    }

    // **Browser support:** The EyeDropper API is Chromium-only (Chrome 95+, Edge 95+).
    // `eyedropper_supported` MUST be set via runtime detection (`"EyeDropper" in window`).
    // When `false`, the EyeDropper trigger MUST NOT be rendered.
    // The adapter MUST call `EyeDropper.open()` directly from a click event handler
    // (within user gesture, not from an async callback) to satisfy the transient
    // activation requirement. The API is not available in Firefox or Safari.

    /// The attributes for the hidden input element.
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(ref name) = self.props.name {
            attrs.set(HtmlAttr::Name, name);
        }
        attrs.set(HtmlAttr::Value, self.ctx.value.get().to_hex(self.ctx.show_alpha));
        attrs
    }
}

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
            Part::Swatch { index, color } => self.swatch_attrs(index, &color),
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

| Part                 | Element                 | Required | Key Attributes                                                                 |
| -------------------- | ----------------------- | -------- | ------------------------------------------------------------------------------ |
| `Root`               | `<div>`                 | yes      | `data-ars-state`, `data-ars-disabled`, `data-ars-readonly`                     |
| `Label`              | `<label>`               | yes      | `for` (trigger ID)                                                             |
| `Control`            | `<div>`                 | yes      |                                                                                |
| `Trigger`            | `<button>`              | yes      | `aria-haspopup="dialog"`, `aria-expanded`, `aria-controls`, `aria-labelledby`  |
| `Content`            | `<div>`                 | yes      | `role="dialog"`, `aria-labelledby`, `data-ars-state`                           |
| `Area`               | `<div>`                 | yes      | `role="group"`                                                                 |
| `AreaThumb`          | `<div>`                 | yes      | `role="application"`, `aria-roledescription`, `aria-valuetext`, `tabindex="0"` |
| `ChannelSlider`      | `<div>`                 | yes      | `role="group"`, `data-ars-channel`                                             |
| `ChannelSliderThumb` | `<div>`                 | yes      | `role="slider"`, `tabindex="0"`, `aria-valuenow`, `aria-label`                 |
| `AlphaSlider`        | `<div>`                 | no       | `role="group"`, `data-ars-channel="alpha"`                                     |
| `SwatchGroup`        | `<div>`                 | no       | `role="group"`                                                                 |
| `Swatch`             | `<button>`              | no       | `role="button"`, `aria-label`, `data-ars-selected`, `data-ars-index`           |
| `FormatSelect`       | `<select>` / `<button>` | no       | `aria-label`                                                                   |
| `ChannelInput`       | `<input>`               | no       | `type="text"`, `inputmode="numeric"`, `data-ars-channel-index`                 |
| `HexInput`           | `<input>`               | no       | `type="text"`, `inputmode="text"`                                              |
| `EyeDropperTrigger`  | `<button>`              | no       | `aria-label`, `hidden` (when unsupported)                                      |
| `HiddenInput`        | `<input type="hidden">` | yes      | `name`, `value`                                                                |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Attribute / Behaviour               | Element                | Value                         |
| ----------------------------------- | ---------------------- | ----------------------------- |
| `role="dialog"`                     | `Content`              | Popover container             |
| `aria-haspopup="dialog"`            | `Trigger`              | Indicates popover             |
| `aria-expanded`                     | `Trigger`              | `"true"` / `"false"`          |
| `aria-controls`                     | `Trigger`              | Content ID                    |
| `aria-labelledby`                   | `Content`, `Trigger`   | Label ID                      |
| `role="application"`                | `AreaThumb`            | 2D color area interaction     |
| `aria-roledescription="color area"` | `AreaThumb`            | Describes the 2D area control |
| `role="slider"`                     | Channel slider thumbs  | Slider interaction            |
| `aria-valuenow`                     | Channel slider thumbs  | Current numeric value         |
| `aria-valuemin` / `aria-valuemax`   | Channel slider thumbs  | Channel range                 |
| `aria-valuetext`                    | `AreaThumb`            | Formatted color string        |
| `aria-label`                        | `AreaThumb`            | `"Color area selector"`       |
| `aria-label`                        | Channel slider thumbs  | `"Hue"`, `"Alpha"`            |
| `aria-label`                        | `EyeDropperTrigger`    | `"Pick color from screen"`    |
| `aria-label`                        | `FormatSelect`         | `"Toggle color format"`       |
| `aria-label`                        | `Swatch`               | `"Select color #rrggbb"`      |
| `aria-live="polite"`                | Value text live region | Announces color changes       |
| `aria-disabled="true"`              | `Trigger`              | When disabled                 |
| `aria-keyshortcuts`                 | `AreaThumb`            | Documents arrow key controls  |

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
#[derive(Clone, Debug)]
pub struct Messages {
    pub trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub area_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub area_role_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub hue_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub alpha_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub saturation_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub lightness_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
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

- The `eyedropper_supported` context field is set via runtime detection (`"EyeDropper" in window`) during the `detect-eyedropper` effect when the picker opens.
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

| Feature                    | ars-ui                    | Ark UI                     | React Aria               | Notes                                                        |
| -------------------------- | ------------------------- | -------------------------- | ------------------------ | ------------------------------------------------------------ |
| `value` / `defaultValue`   | `value` / `default_value` | `value` / `defaultValue`   | `value` / `defaultValue` | Equivalent                                                   |
| `format` / `defaultFormat` | `default_format`          | `format` / `defaultFormat` | --                       | Ark has controlled format; ars-ui uses `Event::SetFormat`    |
| `open` / `defaultOpen`     | `open` / `default_open`   | `open` / `defaultOpen`     | --                       | Equivalent                                                   |
| `disabled`                 | `disabled`                | `disabled`                 | --                       | Equivalent                                                   |
| `readOnly`                 | `readonly`                | `readOnly`                 | --                       | Equivalent                                                   |
| `invalid`                  | --                        | `invalid`                  | --                       | Ark-only; ars-ui validates at form level                     |
| `required`                 | --                        | `required`                 | --                       | Ark-only; ars-ui validates at form level                     |
| `closeOnSelect`            | --                        | `closeOnSelect`            | --                       | Ark-only; ars-ui leaves swatch selection behavior to adapter |
| `inline`                   | --                        | `inline`                   | --                       | Ark-only; ars-ui renders via open state                      |
| `name`                     | `name`                    | `name`                     | --                       | Equivalent                                                   |
| `positioning`              | `positioning`             | `positioning`              | --                       | Equivalent                                                   |
| `colorSpace`               | `color_space`             | --                         | --                       | ars-ui exclusive                                             |
| `showAlpha`                | `show_alpha`              | --                         | --                       | ars-ui exclusive                                             |
| `on_change_end`            | `on_change_end`           | `onValueChangeEnd`         | `onChange`               | Equivalent intent                                            |

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
