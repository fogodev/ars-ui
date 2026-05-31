//! Shared color value types and helpers for the color components.
//!
//! The internal color representation is [`ColorValue`] (HSL + alpha). All other
//! color spaces (RGB, HEX, HSB/HSV, CSS) are computed on demand via conversion
//! methods. These types are the value layer shared by every color component
//! (`ColorSwatch`, `ColorField`, `ColorArea`, `ColorSlider`, `ColorWheel`,
//! `ColorSwatchPicker`, and the future `ColorPicker`); `AngleSlider` is the one
//! exception — it operates on bare `f64` degrees.
//!
//! All color logic lives here in `ars-core` so it is framework-agnostic and
//! `no_std`-compatible. Live geometry (track/area measurement, pointer capture,
//! coordinate-to-value conversion) is supplied by adapters via native handles;
//! this module only owns the channel math, parsing, formatting, perceptual
//! naming, and WCAG contrast utilities.

use alloc::{
    format,
    string::{String, ToString as _},
    vec::Vec,
};

use core_maths::CoreFloat;

/// Minimum of two finite floats. `no_std`-friendly replacement for `f64::min`
/// (which lives in `std`); inputs here are always finite, so NaN handling is
/// not required.
fn fmin(left: f64, right: f64) -> f64 {
    if left < right { left } else { right }
}

/// Maximum of two finite floats. `no_std`-friendly replacement for `f64::max`.
fn fmax(left: f64, right: f64) -> f64 {
    if left > right { left } else { right }
}

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
    /// clamped to `[0.0, 1.0]`.
    #[must_use]
    pub fn new(hue: f64, saturation: f64, lightness: f64, alpha: f64) -> Self {
        debug_assert!(hue.is_finite(), "hue must be finite");
        debug_assert!(saturation.is_finite(), "saturation must be finite");
        debug_assert!(lightness.is_finite(), "lightness must be finite");
        debug_assert!(alpha.is_finite(), "alpha must be finite");

        Self {
            hue: CoreFloat::rem_euclid(hue, 360.0),
            saturation: saturation.clamp(0.0, 1.0),
            lightness: lightness.clamp(0.0, 1.0),
            alpha: alpha.clamp(0.0, 1.0),
        }
    }

    /// Create from an HSL triplet with full alpha.
    #[must_use]
    pub fn from_hsl(hue: f64, saturation: f64, lightness: f64) -> Self {
        Self::new(hue, saturation, lightness, 1.0)
    }

    /// Convert to RGB (0-255 per channel).
    #[must_use]
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        // Standard HSL -> RGB: `chroma` is the colorfulness, `secondary` is the
        // second-largest component, and `lightness_match` lifts the prime
        // components so the result has the target lightness.
        let hue_sextant = self.hue / 60.0;
        let chroma = (1.0 - CoreFloat::abs(2.0 * self.lightness - 1.0)) * self.saturation;
        let secondary = chroma * (1.0 - CoreFloat::abs(hue_sextant % 2.0 - 1.0));
        let lightness_match = self.lightness - chroma / 2.0;

        let (red_prime, green_prime, blue_prime) = match CoreFloat::floor(hue_sextant) as u8 {
            0 => (chroma, secondary, 0.0),
            1 => (secondary, chroma, 0.0),
            2 => (0.0, chroma, secondary),
            3 => (0.0, secondary, chroma),
            4 => (secondary, 0.0, chroma),
            _ => (chroma, 0.0, secondary),
        };

        (
            CoreFloat::round((red_prime + lightness_match) * 255.0) as u8,
            CoreFloat::round((green_prime + lightness_match) * 255.0) as u8,
            CoreFloat::round((blue_prime + lightness_match) * 255.0) as u8,
        )
    }

    /// Convert to RGBA (0-255 per channel, alpha 0-255).
    #[must_use]
    pub fn to_rgba(&self) -> (u8, u8, u8, u8) {
        let (red, green, blue) = self.to_rgb();

        (red, green, blue, CoreFloat::round(self.alpha * 255.0) as u8)
    }

    /// Convert to hex string (6-digit or 8-digit with alpha).
    ///
    /// The alpha component is only emitted when `include_alpha` is set *and*
    /// the color is not fully opaque.
    #[must_use]
    pub fn to_hex(&self, include_alpha: bool) -> String {
        let (red, green, blue) = self.to_rgb();

        if include_alpha && self.alpha < 1.0 {
            let alpha_byte = CoreFloat::round(self.alpha * 255.0) as u8;

            format!("#{red:02x}{green:02x}{blue:02x}{alpha_byte:02x}")
        } else {
            format!("#{red:02x}{green:02x}{blue:02x}")
        }
    }

    /// Convert to CSS `hsl()` or `hsla()` string.
    #[must_use]
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

    /// Convert to HSB/HSV representation `(hue, saturation, brightness)`.
    ///
    /// Hue is shared with the HSL representation; saturation and brightness are
    /// the HSB-space components in `[0.0, 1.0]`.
    #[must_use]
    pub fn to_hsb(&self) -> (f64, f64, f64) {
        let lightness = self.lightness;
        let saturation = self.saturation;

        let brightness = lightness + saturation * fmin(lightness, 1.0 - lightness);

        let hsb_saturation = if brightness == 0.0 {
            0.0
        } else {
            2.0 * (1.0 - lightness / brightness)
        };

        (self.hue, hsb_saturation, brightness)
    }

    /// Create from RGB values (0-255).
    #[must_use]
    pub fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        let red = f64::from(red) / 255.0;
        let green = f64::from(green) / 255.0;
        let blue = f64::from(blue) / 255.0;

        let max_channel = fmax(fmax(red, green), blue);
        let min_channel = fmin(fmin(red, green), blue);

        let lightness = (max_channel + min_channel) / 2.0;

        if max_channel == min_channel {
            return Self::new(0.0, 0.0, lightness, 1.0);
        }

        let delta = max_channel - min_channel;

        let saturation = if lightness > 0.5 {
            delta / (2.0 - max_channel - min_channel)
        } else {
            delta / (max_channel + min_channel)
        };

        let hue = if max_channel == red {
            ((green - blue) / delta + if green < blue { 6.0 } else { 0.0 }) * 60.0
        } else if max_channel == green {
            ((blue - red) / delta + 2.0) * 60.0
        } else {
            ((red - green) / delta + 4.0) * 60.0
        };

        Self::new(hue, saturation, lightness, 1.0)
    }

    /// Parse a hex string (`#rrggbb` or `#rrggbbaa`).
    ///
    /// Returns `None` for any string whose hex digit count is neither 6 nor 8
    /// or that contains non-hex characters.
    #[must_use]
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
                let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;

                Some(Self::from_rgb(red, green, blue))
            }

            8 => {
                let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let alpha_byte = u8::from_str_radix(&hex[6..8], 16).ok()?;

                let mut color = Self::from_rgb(red, green, blue);

                color.alpha = f64::from(alpha_byte) / 255.0;

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

// ── WCAG contrast utilities ─────────────────────────────────────────────

/// Linearize a single sRGB channel (0-255) per the WCAG 2.1 sRGB definition.
fn srgb_to_linear(channel: u8) -> f64 {
    let normalized = f64::from(channel) / 255.0;

    if normalized <= 0.040_45 {
        normalized / 12.92
    } else {
        CoreFloat::powf((normalized + 0.055) / 1.055, 2.4)
    }
}

impl ColorValue {
    /// Compute the relative luminance per the WCAG 2.1 definition.
    ///
    /// Uses the sRGB linearization formula: if `C <= 0.04045`, `C / 12.92`;
    /// else `((C + 0.055) / 1.055)^2.4`. Returns a value in `[0.0, 1.0]`.
    #[must_use]
    pub fn relative_luminance(&self) -> f64 {
        let (red, green, blue) = self.to_rgb();

        0.2126 * srgb_to_linear(red)
            + 0.7152 * srgb_to_linear(green)
            + 0.0722 * srgb_to_linear(blue)
    }

    /// Compute the WCAG 2.1 contrast ratio between two colors.
    ///
    /// Returns a value in `[1.0, 21.0]`. Higher is more contrast.
    #[must_use]
    pub fn contrast_ratio(&self, other: &ColorValue) -> f64 {
        let own_luminance = self.relative_luminance();

        let other_luminance = other.relative_luminance();

        let (lighter, darker) = if own_luminance > other_luminance {
            (own_luminance, other_luminance)
        } else {
            (other_luminance, own_luminance)
        };

        (lighter + 0.05) / (darker + 0.05)
    }

    /// Check if this color passes WCAG AA contrast against another color.
    ///
    /// Normal text: 4.5:1; large text (>= 18pt or >= 14pt bold): 3:1.
    #[must_use]
    pub fn passes_wcag_aa(&self, other: &ColorValue, large_text: bool) -> bool {
        let ratio = self.contrast_ratio(other);

        if large_text {
            ratio >= 3.0
        } else {
            ratio >= 4.5
        }
    }

    /// Check if this color passes WCAG AAA contrast against another color.
    ///
    /// Normal text: 7:1; large text: 4.5:1.
    #[must_use]
    pub fn passes_wcag_aaa(&self, other: &ColorValue, large_text: bool) -> bool {
        let ratio = self.contrast_ratio(other);

        if large_text {
            ratio >= 4.5
        } else {
            ratio >= 7.0
        }
    }
}

// ── Perceptual color naming ─────────────────────────────────────────────

/// Describes a color in human-readable terms for screen readers.
///
/// Components: lightness modifier, chroma modifier, and hue name. Parts are
/// returned as `String` (not `&'static str`) so the i18n layer can substitute
/// localized labels. [`ColorValue::color_name_parts`] returns English keys; a
/// component's `format_name` message maps and orders them per locale.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorNameParts {
    /// e.g., "dark", "light", "very dark", "very light", or "" for medium.
    pub lightness: String,

    /// e.g., "vibrant", "pale", or "" for moderate.
    pub chroma: String,

    /// e.g., "red", "blue", "cyan-blue", "gray", "white", "black".
    pub hue: String,
}

/// Map an OKLCH hue angle (degrees, 0-360) to one of 13 named hue buckets.
fn hue_name(hue_degrees: f64) -> &'static str {
    match hue_degrees {
        degrees if degrees < 35.0 => "red",
        degrees if degrees < 55.0 => "red-orange",
        degrees if degrees < 75.0 => "orange",
        degrees if degrees < 90.0 => "yellow-orange",
        degrees if degrees < 110.0 => "yellow",
        degrees if degrees < 135.0 => "yellow-green",
        degrees if degrees < 165.0 => "green",
        degrees if degrees < 185.0 => "cyan-green",
        degrees if degrees < 205.0 => "cyan",
        degrees if degrees < 240.0 => "cyan-blue",
        degrees if degrees < 280.0 => "blue",
        degrees if degrees < 320.0 => "purple",
        degrees if degrees < 350.0 => "magenta",
        _ => "red",
    }
}

impl ColorValue {
    /// Convert this color to OKLCH `(lightness, chroma, hue_degrees)`.
    ///
    /// OKLCH is a perceptually uniform space; classifying in it yields more
    /// natural color names than classifying in HSL. Uses Björn Ottosson's
    /// `OKLab` transform from linear sRGB.
    fn to_oklch(self) -> (f64, f64, f64) {
        let (red, green, blue) = self.to_rgb();

        let linear_red = srgb_to_linear(red);
        let linear_green = srgb_to_linear(green);
        let linear_blue = srgb_to_linear(blue);

        // LMS cone responses (long / medium / short wavelength).
        let cone_long = 0.412_221_470_8 * linear_red
            + 0.536_332_536_3 * linear_green
            + 0.051_445_992_9 * linear_blue;

        let cone_medium = 0.211_903_498_2 * linear_red
            + 0.680_699_545_1 * linear_green
            + 0.107_396_956_6 * linear_blue;

        let cone_short = 0.088_302_461_9 * linear_red
            + 0.281_718_837_6 * linear_green
            + 0.629_978_700_5 * linear_blue;

        let long_cbrt = CoreFloat::cbrt(cone_long);
        let medium_cbrt = CoreFloat::cbrt(cone_medium);
        let short_cbrt = CoreFloat::cbrt(cone_short);

        // OKLab lightness and the two opponent-color axes.
        let lab_lightness = 0.210_454_255_3 * long_cbrt + 0.793_617_785_0 * medium_cbrt
            - 0.004_072_046_8 * short_cbrt;

        let lab_green_red = 1.977_998_495_1 * long_cbrt - 2.428_592_205_0 * medium_cbrt
            + 0.450_593_709_9 * short_cbrt;

        let lab_blue_yellow = 0.025_904_037_1 * long_cbrt + 0.782_771_766_2 * medium_cbrt
            - 0.808_675_766_0 * short_cbrt;

        let chroma =
            CoreFloat::sqrt(lab_green_red * lab_green_red + lab_blue_yellow * lab_blue_yellow);

        // atan2 returns radians in (-PI, PI]; convert to degrees and wrap to [0, 360).
        let hue_degrees = CoreFloat::rem_euclid(
            CoreFloat::atan2(lab_blue_yellow, lab_green_red) * (180.0 / core::f64::consts::PI),
            360.0,
        );

        (lab_lightness, chroma, hue_degrees)
    }

    /// Returns English color description parts for accessibility.
    ///
    /// The parts (lightness modifier, chroma modifier, hue name) are English
    /// keys; the localized, reordered string is produced by a component's
    /// `format_name` message. Classification runs in OKLCH:
    ///
    /// 1. Lightness → 5 levels: very dark (`<0.2`), dark (`<0.4`), medium/`""`
    ///    (`<0.6`), light (`<0.8`), very light (`>=0.8`).
    /// 2. Chroma → grayish (`<0.04`), moderate/`""` (`<0.12`), vibrant
    ///    (`>=0.12`); moderate chroma with light lightness reads as "pale".
    /// 3. Near-zero chroma collapses to "black"/"gray"/"white" by lightness.
    /// 4. Hue angle → one of 13 named buckets (see [`hue_name`]).
    #[must_use]
    pub fn color_name_parts(&self) -> ColorNameParts {
        let (oklch_lightness, oklch_chroma, oklch_hue) = self.to_oklch();

        // Near-gray: chroma is too low for a meaningful hue name.
        if oklch_chroma < 0.04 {
            let achromatic = if oklch_lightness < 0.2 {
                "black"
            } else if oklch_lightness > 0.85 {
                "white"
            } else {
                "gray"
            };

            return ColorNameParts {
                lightness: String::new(),
                chroma: String::new(),
                hue: achromatic.to_string(),
            };
        }

        let lightness_label = if oklch_lightness < 0.2 {
            "very dark"
        } else if oklch_lightness < 0.4 {
            "dark"
        } else if oklch_lightness < 0.6 {
            ""
        } else if oklch_lightness < 0.8 {
            "light"
        } else {
            "very light"
        };

        let chroma_label = if oklch_chroma > 0.12 {
            "vibrant"
        } else if oklch_lightness > 0.7 {
            "pale"
        } else {
            ""
        };

        ColorNameParts {
            lightness: lightness_label.to_string(),
            chroma: chroma_label.to_string(),
            hue: hue_name(oklch_hue).to_string(),
        }
    }

    /// Convenience: format as an English-order string such as "dark vibrant blue".
    ///
    /// Joins the non-empty [`ColorNameParts`] with single spaces. Intended for
    /// tests and non-localized contexts; localized callers use a component's
    /// `format_name` message instead.
    #[must_use]
    pub fn color_name_en(&self) -> String {
        let parts = self.color_name_parts();

        [
            parts.lightness.as_str(),
            parts.chroma.as_str(),
            parts.hue.as_str(),
        ]
        .iter()
        .filter(|part| !part.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
    }
}

// ── Supporting enums ────────────────────────────────────────────────────

/// The color format currently displayed in a text input area.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorFormat {
    /// The `#rrggbb` / `#rrggbbaa` hex format.
    #[default]
    Hex,

    /// The `rgb()` / `rgba()` format.
    Rgb,

    /// The `hsl()` / `hsla()` format.
    Hsl,

    /// The `hsb()` format.
    Hsb,
}

/// Color space for the picker controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorSpace {
    /// The RGB color space.
    Rgb,

    /// The HSL color space.
    #[default]
    Hsl,

    /// The HSB color space.
    Hsb,

    /// The HWB color space.
    Hwb,
}

/// Individual color channel identifier, used by `ColorArea`, `ColorSlider`,
/// `ColorField`, and `ColorPicker` for per-channel operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorChannel {
    /// The hue channel (0-360 degrees).
    #[default]
    Hue,

    /// The saturation channel (0.0-1.0).
    Saturation,

    /// The lightness channel (0.0-1.0).
    Lightness,

    /// The brightness channel (HSB/HSV value component, 0.0-1.0).
    Brightness,

    /// The alpha channel (0.0-1.0).
    Alpha,

    /// The red channel (0-255).
    Red,

    /// The green channel (0-255).
    Green,

    /// The blue channel (0-255).
    Blue,
}

/// Identifies what the user is dragging in the `ColorPicker`.
///
/// The `ColorPicker` area targets two channels at once, while channel sliders
/// target a single channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragTarget {
    /// 2D area: `x_channel = Saturation`, `y_channel = Lightness` (fixed in `ColorPicker`).
    Area,

    /// 1D channel slider.
    Channel(ColorChannel),
}

// ── Channel helpers ─────────────────────────────────────────────────────

/// Get the current value of a single channel from a [`ColorValue`].
#[must_use]
pub fn channel_value(color: &ColorValue, channel: ColorChannel) -> f64 {
    match channel {
        ColorChannel::Hue => color.hue,
        ColorChannel::Saturation => color.saturation,
        ColorChannel::Lightness => color.lightness,
        ColorChannel::Brightness => color.to_hsb().2,
        ColorChannel::Alpha => color.alpha,
        ColorChannel::Red => f64::from(color.to_rgb().0),
        ColorChannel::Green => f64::from(color.to_rgb().1),
        ColorChannel::Blue => f64::from(color.to_rgb().2),
    }
}

/// Return a new [`ColorValue`] with one channel replaced.
#[must_use]
pub fn with_channel(color: &ColorValue, channel: ColorChannel, value: f64) -> ColorValue {
    match channel {
        ColorChannel::Hue => ColorValue::new(value, color.saturation, color.lightness, color.alpha),

        ColorChannel::Saturation => ColorValue::new(color.hue, value, color.lightness, color.alpha),

        ColorChannel::Lightness => ColorValue::new(color.hue, color.saturation, value, color.alpha),

        ColorChannel::Brightness => {
            // Convert current HSL to HSB, replace brightness, convert back.
            let (hue, hsb_saturation, _) = color.to_hsb();

            let brightness = value.clamp(0.0, 1.0);

            let new_lightness = brightness * (1.0 - hsb_saturation / 2.0);

            let new_saturation = if new_lightness == 0.0 || new_lightness == 1.0 {
                0.0
            } else {
                (brightness - new_lightness) / fmin(new_lightness, 1.0 - new_lightness)
            };

            ColorValue::new(hue, new_saturation, new_lightness, color.alpha)
        }

        ColorChannel::Alpha => ColorValue::new(color.hue, color.saturation, color.lightness, value),

        ColorChannel::Red => {
            let (_, green, blue) = color.to_rgb();

            let mut updated = ColorValue::from_rgb(CoreFloat::round(value) as u8, green, blue);

            updated.alpha = color.alpha;

            updated
        }

        ColorChannel::Green => {
            let (red, _, blue) = color.to_rgb();

            let mut updated = ColorValue::from_rgb(red, CoreFloat::round(value) as u8, blue);

            updated.alpha = color.alpha;

            updated
        }

        ColorChannel::Blue => {
            let (red, green, _) = color.to_rgb();

            let mut updated = ColorValue::from_rgb(red, green, CoreFloat::round(value) as u8);

            updated.alpha = color.alpha;

            updated
        }
    }
}

/// The `(min, max)` range for a channel.
#[must_use]
pub const fn channel_range(channel: ColorChannel) -> (f64, f64) {
    match channel {
        ColorChannel::Hue => (0.0, 360.0),
        ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue => (0.0, 255.0),
        // Saturation, Lightness, Brightness, Alpha
        _ => (0.0, 1.0),
    }
}

/// The default step size for keyboard adjustment of a channel.
#[must_use]
pub const fn channel_step_default(channel: ColorChannel) -> f64 {
    match channel {
        // 1 degree for hue, 1 unit for the 0-255 RGB channels.
        ColorChannel::Hue | ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue => 1.0,
        // 1% for the 0..1 range channels.
        _ => 0.01,
    }
}

// ── String parsing / formatting ─────────────────────────────────────────

/// Strip a functional notation call: `name(...)` -> inner content.
fn strip_fn_call<'a>(input: &'a str, name: &str) -> Option<&'a str> {
    input
        .strip_prefix(name)?
        .strip_prefix('(')?
        .strip_suffix(')')
}

/// Parse a finite `f64`, rejecting `NaN`/`inf`/`-inf`.
///
/// `str::parse::<f64>` accepts the IEEE-754 special-value spellings, which would
/// otherwise flow into [`ColorValue`] components and surface as a debug-build
/// panic (via the finiteness assertions in [`ColorValue::new`]) or as NaN/inf
/// leaking into generated styles and ARIA values in release builds.
fn parse_finite_f64(input: &str) -> Option<f64> {
    input.parse::<f64>().ok().filter(|value| value.is_finite())
}

/// Parse `r, g, b` or `r, g, b, a` inside `rgb()` / `rgba()`.
fn parse_rgb_args(inner: &str, has_alpha: bool) -> Option<ColorValue> {
    let parts = inner.split(',').map(str::trim).collect::<Vec<_>>();

    if has_alpha && parts.len() != 4 {
        return None;
    }

    if !has_alpha && parts.len() != 3 {
        return None;
    }

    let red = parts[0].parse::<u8>().ok()?;
    let green = parts[1].parse::<u8>().ok()?;
    let blue = parts[2].parse::<u8>().ok()?;

    let alpha = if has_alpha {
        parse_finite_f64(parts[3])?
    } else {
        1.0
    };

    let mut color = ColorValue::from_rgb(red, green, blue);

    color.alpha = alpha.clamp(0.0, 1.0);

    Some(color)
}

/// Parse `h, s%, l%` or `h, s%, l%, a` inside `hsl()` / `hsla()`.
fn parse_hsl_args(inner: &str, has_alpha: bool) -> Option<ColorValue> {
    let parts = inner.split(',').map(str::trim).collect::<Vec<_>>();

    if has_alpha && parts.len() != 4 {
        return None;
    }

    if !has_alpha && parts.len() != 3 {
        return None;
    }

    let hue = parse_finite_f64(parts[0])?;
    let saturation = parse_finite_f64(parts[1].strip_suffix('%')?.trim())? / 100.0;
    let lightness = parse_finite_f64(parts[2].strip_suffix('%')?.trim())? / 100.0;

    let alpha = if has_alpha {
        parse_finite_f64(parts[3])?
    } else {
        1.0
    };

    Some(ColorValue::new(
        hue,
        saturation,
        lightness,
        alpha.clamp(0.0, 1.0),
    ))
}

/// Parse `h, s%, b%` inside `hsb()`.
fn parse_hsb_args(inner: &str) -> Option<ColorValue> {
    let parts = inner.split(',').map(str::trim).collect::<Vec<_>>();

    if parts.len() != 3 {
        return None;
    }

    let hue = parse_finite_f64(parts[0])?;
    let saturation = parse_finite_f64(parts[1].strip_suffix('%')?.trim())? / 100.0;
    let brightness = parse_finite_f64(parts[2].strip_suffix('%')?.trim())? / 100.0;

    // Convert HSB -> HSL: lightness = brightness * (1 - saturation/2).
    let lightness = brightness * (1.0 - saturation / 2.0);

    let hsl_saturation = if lightness > 0.0 && lightness < 1.0 {
        (brightness - lightness) / fmin(lightness, 1.0 - lightness)
    } else {
        0.0
    };

    Some(ColorValue::new(hue, hsl_saturation, lightness, 1.0))
}

/// Parse a user-typed string into a [`ColorValue`].
///
/// Recognizes: `#rrggbb`, `#rrggbbaa`, `rgb(r,g,b)`, `rgba(r,g,b,a)`,
/// `hsl(h,s%,l%)`, `hsla(h,s%,l%,a)`, and `hsb(h,s%,b%)`. Returns `None` for
/// any unrecognized or malformed input.
#[must_use]
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

/// Format a [`ColorValue`] as a string in the given format.
#[must_use]
pub fn format_color_string(color: &ColorValue, format: ColorFormat) -> String {
    match format {
        ColorFormat::Hex => color.to_hex(color.alpha < 1.0),

        ColorFormat::Rgb => {
            let (red, green, blue) = color.to_rgb();

            if color.alpha < 1.0 {
                format!("rgba({red}, {green}, {blue}, {:.2})", color.alpha)
            } else {
                format!("rgb({red}, {green}, {blue})")
            }
        }

        ColorFormat::Hsl => color.to_css_hsl(),

        ColorFormat::Hsb => {
            let (hue, saturation, brightness) = color.to_hsb();

            format!(
                "hsb({hue:.0}, {:.1}%, {:.1}%)",
                saturation * 100.0,
                brightness * 100.0
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip helper: assert two RGB triples are within `tolerance` per channel.
    fn rgb_close(actual: (u8, u8, u8), expected: (u8, u8, u8), tolerance: u8) {
        assert!(
            actual.0.abs_diff(expected.0) <= tolerance
                && actual.1.abs_diff(expected.1) <= tolerance
                && actual.2.abs_diff(expected.2) <= tolerance,
            "rgb {actual:?} not within {tolerance} of {expected:?}"
        );
    }

    #[test]
    fn default_is_pure_red() {
        let color = ColorValue::default();

        assert_eq!(color.to_rgb(), (255, 0, 0));
        assert_eq!(color.alpha, 1.0);
    }

    #[test]
    fn new_wraps_hue_and_clamps_channels() {
        let color = ColorValue::new(400.0, 2.0, -1.0, 5.0);

        assert!((color.hue - 40.0).abs() < 1e-9);
        assert_eq!(color.saturation, 1.0);
        assert_eq!(color.lightness, 0.0);
        assert_eq!(color.alpha, 1.0);
    }

    #[test]
    fn hsl_rgb_round_trip_primaries() {
        for rgb in [
            (255, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (255, 255, 0),
            (0, 255, 255),
            (255, 0, 255),
            (128, 64, 32),
        ] {
            let color = ColorValue::from_rgb(rgb.0, rgb.1, rgb.2);

            rgb_close(color.to_rgb(), rgb, 1);
        }
    }

    #[test]
    fn hex_parse_and_format() {
        let color = ColorValue::from_hex("#3366ff").unwrap();

        assert_eq!(color.to_rgb(), (0x33, 0x66, 0xff));
        assert_eq!(color.to_hex(false), "#3366ff");

        let with_alpha = ColorValue::from_hex("#3366ff80").unwrap();

        assert!((with_alpha.alpha - 128.0 / 255.0).abs() < 1e-9);
        assert_eq!(with_alpha.to_hex(true), "#3366ff80");

        assert_eq!(ColorValue::from_hex("nope"), None);
        assert_eq!(ColorValue::from_hex("#fff"), None);
    }

    #[test]
    fn parse_color_string_recognizes_all_formats() {
        rgb_close(
            parse_color_string("rgb(255, 0, 0)").unwrap().to_rgb(),
            (255, 0, 0),
            1,
        );

        let rgba = parse_color_string("rgba(0, 0, 255, 0.5)").unwrap();

        rgb_close(rgba.to_rgb(), (0, 0, 255), 1);

        assert!((rgba.alpha - 0.5).abs() < 1e-9);

        rgb_close(
            parse_color_string("hsl(120, 100%, 50%)").unwrap().to_rgb(),
            (0, 255, 0),
            1,
        );

        let hsla = parse_color_string("hsla(0, 100%, 50%, 0.25)").unwrap();

        assert!((hsla.alpha - 0.25).abs() < 1e-9);

        rgb_close(
            parse_color_string("hsb(240, 100%, 100%)").unwrap().to_rgb(),
            (0, 0, 255),
            1,
        );

        assert_eq!(parse_color_string("not a color"), None);
        assert_eq!(parse_color_string("rgb(1, 2)"), None);
    }

    #[test]
    fn format_color_string_per_format() {
        let red = ColorValue::from_rgb(255, 0, 0);

        assert_eq!(format_color_string(&red, ColorFormat::Hex), "#ff0000");
        assert_eq!(
            format_color_string(&red, ColorFormat::Rgb),
            "rgb(255, 0, 0)"
        );
        assert_eq!(
            format_color_string(&red, ColorFormat::Hsl),
            "hsl(0, 100.0%, 50.0%)"
        );

        let translucent = ColorValue::new(0.0, 1.0, 0.5, 0.5);

        assert_eq!(
            format_color_string(&translucent, ColorFormat::Rgb),
            "rgba(255, 0, 0, 0.50)"
        );
    }

    #[test]
    fn channel_helpers_round_trip() {
        let color = ColorValue::from_hsl(200.0, 0.5, 0.5);

        assert!((channel_value(&color, ColorChannel::Hue) - 200.0).abs() < 1e-9);

        let updated = with_channel(&color, ColorChannel::Hue, 90.0);

        assert!((channel_value(&updated, ColorChannel::Hue) - 90.0).abs() < 1e-9);

        // Replacing alpha leaves hue/saturation/lightness untouched.
        let dimmed = with_channel(&color, ColorChannel::Alpha, 0.25);

        assert!((dimmed.alpha - 0.25).abs() < 1e-9);
        assert!((dimmed.hue - color.hue).abs() < 1e-9);

        assert_eq!(channel_range(ColorChannel::Hue), (0.0, 360.0));
        assert_eq!(channel_range(ColorChannel::Red), (0.0, 255.0));
        assert_eq!(channel_range(ColorChannel::Saturation), (0.0, 1.0));
        assert!((channel_step_default(ColorChannel::Hue) - 1.0).abs() < 1e-9);
        assert!((channel_step_default(ColorChannel::Saturation) - 0.01).abs() < 1e-9);
    }

    #[test]
    fn with_channel_rgb_replacement() {
        let color = ColorValue::from_rgb(10, 20, 30);

        let red = with_channel(&color, ColorChannel::Red, 200.0);

        rgb_close(red.to_rgb(), (200, 20, 30), 1);
    }

    #[test]
    fn wcag_contrast_black_on_white_is_maximal() {
        let black = ColorValue::from_rgb(0, 0, 0);
        let white = ColorValue::from_rgb(255, 255, 255);

        let ratio = black.contrast_ratio(&white);

        assert!((ratio - 21.0).abs() < 0.1, "ratio was {ratio}");
        assert!(black.passes_wcag_aaa(&white, false));
        assert!(white.passes_wcag_aa(&black, false));

        // A mid gray on white fails AA for normal text.
        let gray = ColorValue::from_rgb(150, 150, 150);

        assert!(!gray.passes_wcag_aa(&white, false));
    }

    #[test]
    fn perceptual_names_for_known_colors() {
        assert!(
            ColorValue::from_rgb(255, 0, 0)
                .color_name_en()
                .contains("red")
        );
        assert!(
            ColorValue::from_rgb(0, 0, 255)
                .color_name_en()
                .contains("blue")
        );
        assert!(
            ColorValue::from_rgb(0, 200, 0)
                .color_name_en()
                .contains("green")
        );
        assert_eq!(ColorValue::from_rgb(0, 0, 0).color_name_en(), "black");
        assert_eq!(ColorValue::from_rgb(255, 255, 255).color_name_en(), "white");
        assert_eq!(ColorValue::from_rgb(128, 128, 128).color_name_en(), "gray");
    }

    #[test]
    fn color_name_parts_are_english_keys() {
        // Gray collapses to an achromatic hue with no lightness/chroma modifier.
        let parts = ColorValue::from_rgb(128, 128, 128).color_name_parts();

        assert_eq!(parts.lightness, "");
        assert_eq!(parts.chroma, "");
        assert_eq!(parts.hue, "gray");
    }

    #[test]
    fn to_rgba_and_css_hsl_cover_alpha() {
        let color = ColorValue::new(0.0, 1.0, 0.5, 0.5);

        let (red, green, blue, alpha) = color.to_rgba();

        assert_eq!((red, green, blue), (255, 0, 0));
        assert_eq!(alpha, 128);
        // hsla branch (alpha < 1).
        assert_eq!(color.to_css_hsl(), "hsla(0, 100.0%, 50.0%, 0.50)");
        // hsl branch (opaque).
        assert_eq!(
            ColorValue::from_hsl(0.0, 1.0, 0.5).to_css_hsl(),
            "hsl(0, 100.0%, 50.0%)"
        );
    }

    #[test]
    fn to_hsb_and_brightness_channel() {
        // Pure red: HSB brightness is 1.0, saturation 1.0.
        let (hue, saturation, brightness) = ColorValue::from_hsl(0.0, 1.0, 0.5).to_hsb();

        assert!((hue - 0.0).abs() < 1e-9);
        assert!((saturation - 1.0).abs() < 1e-9);
        assert!((brightness - 1.0).abs() < 1e-9);
        // Black has brightness 0.
        assert_eq!(ColorValue::from_rgb(0, 0, 0).to_hsb().2, 0.0);

        // Round-trip brightness through with_channel.
        let dimmed = with_channel(
            &ColorValue::from_hsl(0.0, 1.0, 0.5),
            ColorChannel::Brightness,
            0.5,
        );

        assert!((channel_value(&dimmed, ColorChannel::Brightness) - 0.5).abs() < 0.02);
    }

    #[test]
    fn channel_value_reads_every_channel() {
        let color = ColorValue::new(120.0, 0.4, 0.6, 0.8);

        assert!((channel_value(&color, ColorChannel::Hue) - 120.0).abs() < 1e-9);
        assert!((channel_value(&color, ColorChannel::Saturation) - 0.4).abs() < 1e-9);
        assert!((channel_value(&color, ColorChannel::Lightness) - 0.6).abs() < 1e-9);
        assert!((channel_value(&color, ColorChannel::Alpha) - 0.8).abs() < 1e-9);
        assert!(channel_value(&color, ColorChannel::Brightness) > 0.0);

        let (red, green, blue) = color.to_rgb();

        assert_eq!(channel_value(&color, ColorChannel::Red) as u8, red);
        assert_eq!(channel_value(&color, ColorChannel::Green) as u8, green);
        assert_eq!(channel_value(&color, ColorChannel::Blue) as u8, blue);
    }

    #[test]
    fn with_channel_replaces_every_channel() {
        let base = ColorValue::new(120.0, 0.4, 0.6, 0.8);

        assert!((with_channel(&base, ColorChannel::Saturation, 0.1).saturation - 0.1).abs() < 1e-9);
        assert!((with_channel(&base, ColorChannel::Lightness, 0.2).lightness - 0.2).abs() < 1e-9);

        // Green/Blue replacement preserves alpha.
        let with_green = with_channel(&base, ColorChannel::Green, 10.0);

        assert_eq!(with_green.to_rgb().1, 10);
        assert!((with_green.alpha - 0.8).abs() < 1e-9);

        let with_blue = with_channel(&base, ColorChannel::Blue, 200.0);

        assert_eq!(with_blue.to_rgb().2, 200);
    }

    #[test]
    fn perceptual_naming_covers_lightness_and_chroma_modifiers() {
        // A dark vivid color picks up a lightness modifier and "vibrant".
        let dark_blue = ColorValue::from_rgb(0, 0, 90).color_name_en();

        assert!(dark_blue.contains("dark"), "got {dark_blue:?}");
        assert!(dark_blue.contains("blue"), "got {dark_blue:?}");

        // A light pastel reads as light + a hue.
        let pastel = ColorValue::from_rgb(255, 200, 200).color_name_en();

        assert!(
            pastel.contains("light") || pastel.contains("pale"),
            "got {pastel:?}"
        );
    }

    #[test]
    fn hue_name_covers_every_bucket() {
        // One representative angle per named bucket (deterministic, no OKLCH guesswork).
        assert_eq!(hue_name(10.0), "red");
        assert_eq!(hue_name(45.0), "red-orange");
        assert_eq!(hue_name(65.0), "orange");
        assert_eq!(hue_name(82.0), "yellow-orange");
        assert_eq!(hue_name(100.0), "yellow");
        assert_eq!(hue_name(120.0), "yellow-green");
        assert_eq!(hue_name(150.0), "green");
        assert_eq!(hue_name(175.0), "cyan-green");
        assert_eq!(hue_name(195.0), "cyan");
        assert_eq!(hue_name(220.0), "cyan-blue");
        assert_eq!(hue_name(260.0), "blue");
        assert_eq!(hue_name(300.0), "purple");
        assert_eq!(hue_name(335.0), "magenta");
        // Wraparound back to red.
        assert_eq!(hue_name(355.0), "red");
    }

    #[test]
    fn wcag_large_text_thresholds() {
        // ~3.4:1 passes AA-large (3:1) but fails AA-normal (4.5:1).
        let fg = ColorValue::from_rgb(140, 140, 140);
        let bg = ColorValue::from_rgb(255, 255, 255);

        assert!(fg.passes_wcag_aa(&bg, true));
        assert!(!fg.passes_wcag_aa(&bg, false));

        // Black/white passes AAA at both text sizes.
        let black = ColorValue::from_rgb(0, 0, 0);

        assert!(black.passes_wcag_aaa(&bg, true));
        assert!(black.passes_wcag_aaa(&bg, false));
    }

    #[test]
    fn format_color_string_hsb_branch() {
        let red = ColorValue::from_hsl(0.0, 1.0, 0.5);

        assert_eq!(
            format_color_string(&red, ColorFormat::Hsb),
            "hsb(0, 100.0%, 100.0%)"
        );
    }

    #[test]
    fn parse_rejects_malformed_functional_notation() {
        assert_eq!(parse_color_string("rgb(300, 0, 0)"), None); // 300 > u8::MAX
        assert_eq!(parse_color_string("hsl(0, 100, 50%)"), None); // missing % on saturation
        assert_eq!(parse_color_string("hsb(0, 100%, x%)"), None); // non-numeric
        assert_eq!(parse_color_string("rgba(0,0,0)"), None); // alpha arity mismatch
        assert_eq!(parse_color_string("hsl(0, 50%)"), None); // too few hsl args
        assert_eq!(parse_color_string("hsb(0, 50%)"), None); // too few hsb args
    }

    #[test]
    fn parse_color_string_dispatches_hex() {
        // The `#` branch of the dispatcher delegates to `from_hex`.
        assert_eq!(parse_color_string("#00ff00").unwrap().to_rgb(), (0, 255, 0));
    }

    #[test]
    fn from_hex_rejects_extra_leading_markers() {
        // Only one optional leading `#` is allowed; `##3366ff` is malformed.
        assert_eq!(parse_color_string("##3366ff"), None);
        assert_eq!(ColorValue::from_hex("##3366ff"), None);
        // The single-marker and bare forms still parse.
        assert_eq!(
            ColorValue::from_hex("#3366ff").unwrap().to_rgb(),
            (51, 102, 255)
        );
        assert_eq!(
            ColorValue::from_hex("3366ff").unwrap().to_rgb(),
            (51, 102, 255)
        );
    }

    #[test]
    fn from_hex_rejects_non_ascii_without_panicking() {
        // A `#`-prefixed string whose byte length is 6 or 8 but whose bytes are
        // not ASCII (so the 2-byte slices fall on non-char boundaries) must
        // return None rather than panic. Each `あ` is 3 bytes, so `#ああ`
        // trims to a 6-byte string.
        assert_eq!(ColorValue::from_hex("#ああ"), None);
        assert_eq!(parse_color_string("#ああ"), None);
        // 8-byte non-ASCII variant (`あ`+`あ`+`é` = 3+3+2 bytes): the leading
        // 2-byte slice falls inside the first multi-byte char.
        assert_eq!(parse_color_string("#ああé"), None);
    }

    #[test]
    fn parse_rejects_non_finite_components() {
        // `f64::parse` accepts "NaN"/"inf"/"-inf", which would otherwise build a
        // `ColorValue` with non-finite components: a debug-build panic via the
        // `ColorValue::new` finiteness assertions, and NaN/inf leaking into
        // generated styles and ARIA values in release builds.
        assert_eq!(parse_color_string("hsl(NaN, 100%, 50%)"), None);
        assert_eq!(parse_color_string("hsl(inf, 100%, 50%)"), None);
        assert_eq!(parse_color_string("hsl(0, NaN%, 50%)"), None);
        assert_eq!(parse_color_string("hsl(0, 100%, inf%)"), None);
        assert_eq!(parse_color_string("hsla(0, 100%, 50%, NaN)"), None);
        assert_eq!(parse_color_string("hsb(NaN, 100%, 100%)"), None);
        assert_eq!(parse_color_string("hsb(0, inf%, 100%)"), None);
        assert_eq!(parse_color_string("rgba(0, 0, 0, NaN)"), None);
        assert_eq!(parse_color_string("rgba(0, 0, 0, inf)"), None);
    }
}
