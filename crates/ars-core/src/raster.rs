//! Platform-agnostic interface for rasterizing vector strokes into an encoded
//! image (PNG/JPEG).
//!
//! The agnostic core models a signature (and similar freehand input) as vector
//! strokes. Turning those into a raster image requires a pixel surface and an
//! encoder, which only the platform provides — so, like
//! [`PlatformEffects`](crate::PlatformEffects), rasterization goes through a
//! trait resolved from the adapter layer:
//!
//! - `ars-dom` provides `WebSignatureRasterizer` (browser `<canvas>`).
//! - [`NullSignatureRasterizer`] is the no-op for SSR and unit tests.
//!
//! The contract is deliberately neutral — it takes pressure-weighted points and
//! a [`RasterSpec`], never any component-specific type — so the lower-level
//! `ars-dom` crate can implement it without depending on the component crates.

use alloc::{string::String, vec::Vec};
use core::fmt::{self, Display};

/// A sampled stroke point for rasterization: position plus normalized pressure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasterPoint {
    /// The x coordinate of the point, in canvas pixels.
    pub x: f64,

    /// The y coordinate of the point, in canvas pixels.
    pub y: f64,

    /// Normalized pressure in `0.0..=1.0`; scales the rendered stroke width so
    /// firmer presses draw thicker segments.
    pub pressure: f64,
}

/// Encoded image format produced by a [`SignatureRasterizer`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RasterFormat {
    /// PNG (lossless, supports transparency).
    Png,

    /// JPEG (lossy; honors [`RasterSpec::quality`], has no alpha channel).
    Jpeg,

    /// WebP (honors [`RasterSpec::quality`]). Not encodable by every browser
    /// canvas — Safari silently falls back to PNG — so a rasterizer requesting
    /// WebP MUST report the format it actually produced (see
    /// [`RasterImage::format`]), not the requested one.
    Webp,
}

impl RasterFormat {
    /// The MIME type string used to request this format from a canvas encoder.
    #[must_use]
    pub const fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
        }
    }

    /// Parses a MIME type — e.g. the one embedded in a `data:` URL returned by a
    /// canvas — back into a [`RasterFormat`]. Returns `None` for unrecognized
    /// types, letting callers fall back to the requested format.
    #[must_use]
    pub fn from_mime(mime: &str) -> Option<Self> {
        match mime {
            "image/png" => Some(Self::Png),
            "image/jpeg" => Some(Self::Jpeg),
            "image/webp" => Some(Self::Webp),
            _ => None,
        }
    }
}

/// Rendering parameters for a rasterization request.
#[derive(Clone, Debug, PartialEq)]
pub struct RasterSpec {
    /// Output bitmap width in pixels.
    pub width: u32,

    /// Output bitmap height in pixels.
    pub height: u32,

    /// Stroke color as a CSS color string.
    pub pen_color: String,

    /// Base stroke width in pixels (scaled per point by [`RasterPoint::pressure`]).
    pub pen_width: f64,

    /// Background fill as a CSS color string. `None` leaves the surface
    /// transparent (PNG); JPEG has no alpha, so a `None` background renders
    /// as the platform default (typically black).
    pub background: Option<String>,

    /// Encoded output format.
    pub format: RasterFormat,

    /// Encoder quality in `0.0..=1.0` for [`RasterFormat::Jpeg`]; ignored for PNG.
    pub quality: f64,
}

impl RasterSpec {
    /// Creates a [`RasterSpec`] for a `width` × `height` PNG with the default
    /// black pen, 2px width, transparent background, and 0.92 JPEG quality.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pen_color: String::from("#000000"),
            pen_width: 2.0,
            background: None,
            format: RasterFormat::Png,
            quality: 0.92,
        }
    }

    /// Sets [`pen_color`](Self::pen_color).
    #[must_use]
    pub fn pen_color(mut self, pen_color: impl Into<String>) -> Self {
        self.pen_color = pen_color.into();
        self
    }

    /// Sets [`pen_width`](Self::pen_width).
    #[must_use]
    pub const fn pen_width(mut self, pen_width: f64) -> Self {
        self.pen_width = pen_width;
        self
    }

    /// Sets [`background`](Self::background) to an opaque CSS color.
    #[must_use]
    pub fn background(mut self, background: impl Into<String>) -> Self {
        self.background = Some(background.into());
        self
    }

    /// Sets [`format`](Self::format).
    #[must_use]
    pub const fn format(mut self, format: RasterFormat) -> Self {
        self.format = format;
        self
    }

    /// Sets [`quality`](Self::quality) (JPEG only).
    #[must_use]
    pub const fn quality(mut self, quality: f64) -> Self {
        self.quality = quality;
        self
    }
}

/// An encoded raster image produced by a [`SignatureRasterizer`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RasterImage {
    /// The format the bytes are encoded in.
    pub format: RasterFormat,

    /// The encoded image bytes (e.g. PNG/JPEG file contents).
    pub bytes: Vec<u8>,
}

/// Why a rasterization request failed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RasterError {
    /// No rasterization backend is available in this environment (SSR, unit
    /// tests, or a non-browser target). Returned by [`NullSignatureRasterizer`].
    Unsupported,

    /// The platform rasterization pipeline failed (e.g. the canvas context or
    /// the image encoder was unavailable). Carries a human-readable reason.
    Backend(String),
}

impl Display for RasterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => {
                f.write_str("signature rasterization is not supported in this environment")
            }
            Self::Backend(reason) => write!(f, "signature rasterization failed: {reason}"),
        }
    }
}

impl core::error::Error for RasterError {}

/// Platform capability that rasterizes vector strokes into an encoded image.
///
/// Adapters supply an implementation (`ars-dom`'s `WebSignatureRasterizer` for
/// the browser); [`NullSignatureRasterizer`] is the SSR/test no-op. Requires
/// `Send + Sync` to mirror [`PlatformEffects`](crate::PlatformEffects) so it can
/// be shared via [`Arc`](alloc::sync::Arc) on native targets.
pub trait SignatureRasterizer: Send + Sync {
    /// Rasterizes `strokes` — each a polyline of pressure-weighted
    /// [`RasterPoint`]s — into an encoded image per `spec`.
    ///
    /// # Errors
    ///
    /// Returns [`RasterError::Unsupported`] when the environment has no
    /// rasterization backend, or [`RasterError::Backend`] when the platform
    /// pipeline fails.
    fn rasterize(
        &self,
        strokes: &[Vec<RasterPoint>],
        spec: &RasterSpec,
    ) -> Result<RasterImage, RasterError>;
}

/// No-op [`SignatureRasterizer`] for SSR and unit tests.
///
/// Always returns [`RasterError::Unsupported`]; there is no pixel surface to
/// rasterize onto outside a browser.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullSignatureRasterizer;

impl SignatureRasterizer for NullSignatureRasterizer {
    fn rasterize(
        &self,
        _strokes: &[Vec<RasterPoint>],
        _spec: &RasterSpec,
    ) -> Result<RasterImage, RasterError> {
        Err(RasterError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn raster_spec_new_defaults_match_signature_pad() {
        let spec = RasterSpec::new(300, 150);

        assert_eq!(spec.width, 300);
        assert_eq!(spec.height, 150);
        assert_eq!(spec.pen_color, "#000000");
        assert_eq!(spec.pen_width, 2.0);
        assert_eq!(spec.background, None);
        assert_eq!(spec.format, RasterFormat::Png);
        assert_eq!(spec.quality, 0.92);
    }

    #[test]
    fn raster_spec_builder_sets_fields() {
        let spec = RasterSpec::new(10, 20)
            .pen_color("#ff0000")
            .pen_width(4.0)
            .background("#ffffff")
            .format(RasterFormat::Jpeg)
            .quality(0.5);

        assert_eq!(spec.pen_color, "#ff0000");
        assert_eq!(spec.pen_width, 4.0);
        assert_eq!(spec.background.as_deref(), Some("#ffffff"));
        assert_eq!(spec.format, RasterFormat::Jpeg);
        assert_eq!(spec.quality, 0.5);
    }

    #[test]
    fn raster_format_mime_round_trips() {
        for format in [RasterFormat::Png, RasterFormat::Jpeg, RasterFormat::Webp] {
            assert_eq!(RasterFormat::from_mime(format.mime()), Some(format));
        }

        assert_eq!(RasterFormat::Png.mime(), "image/png");
        assert_eq!(RasterFormat::Jpeg.mime(), "image/jpeg");
        assert_eq!(RasterFormat::Webp.mime(), "image/webp");
    }

    #[test]
    fn raster_format_from_unknown_mime_is_none() {
        assert_eq!(RasterFormat::from_mime("image/gif"), None);
        assert_eq!(RasterFormat::from_mime(""), None);
    }

    #[test]
    fn null_rasterizer_reports_unsupported() {
        let strokes = vec![vec![
            RasterPoint {
                x: 0.0,
                y: 0.0,
                pressure: 0.5,
            },
            RasterPoint {
                x: 1.0,
                y: 1.0,
                pressure: 0.5,
            },
        ]];

        let result = NullSignatureRasterizer.rasterize(&strokes, &RasterSpec::new(10, 10));

        assert_eq!(result, Err(RasterError::Unsupported));
    }

    #[test]
    fn raster_error_display_is_human_readable() {
        use alloc::string::ToString as _;

        assert_eq!(
            RasterError::Unsupported.to_string(),
            "signature rasterization is not supported in this environment"
        );
        assert_eq!(
            RasterError::Backend("no 2d context".into()).to_string(),
            "signature rasterization failed: no 2d context"
        );
    }

    #[test]
    fn raster_error_implements_std_error() {
        fn assert_error<E: core::error::Error>(_: &E) {}

        assert_error(&RasterError::Unsupported);
    }

    #[test]
    fn raster_types_clone_debug_and_compare() {
        use alloc::format;

        let point = RasterPoint {
            x: 1.0,
            y: 2.0,
            pressure: 0.3,
        };

        assert_eq!(point, point.clone());
        assert!(format!("{point:?}").contains("RasterPoint"));

        let spec = RasterSpec::new(10, 20).background("#ffffff");

        assert_eq!(spec, spec.clone());
        assert!(format!("{spec:?}").contains("RasterSpec"));

        let image = RasterImage {
            format: RasterFormat::Webp,
            bytes: vec![1, 2, 3],
        };

        assert_eq!(image, image.clone());
        assert!(format!("{image:?}").contains("RasterImage"));

        let error = RasterError::Backend("boom".into());

        assert_eq!(error, error.clone());
        assert!(format!("{error:?}").contains("Backend"));

        // Exercise every `RasterFormat` Debug arm and inequality.
        assert_eq!(format!("{:?}", RasterFormat::Png), "Png");
        assert_eq!(format!("{:?}", RasterFormat::Jpeg), "Jpeg");
        assert_eq!(format!("{:?}", RasterFormat::Webp), "Webp");
        assert_ne!(RasterFormat::Png, RasterFormat::Webp);

        // Differing specs compare unequal (covers the comparison's false paths).
        assert_ne!(spec, RasterSpec::new(10, 20));
        assert_ne!(RasterError::Unsupported, RasterError::Backend("x".into()));

        let rasterizer = NullSignatureRasterizer;
        let copied = rasterizer;

        assert!(format!("{copied:?}").contains("NullSignatureRasterizer"));
    }
}
