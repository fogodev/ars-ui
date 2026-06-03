//! Browser-backed [`ars_core::SignatureRasterizer`] implementation.
//!
//! Renders signature strokes onto an offscreen `<canvas>` and encodes the
//! result with `canvas.toDataURL`, decoding the data URL back to raw image
//! bytes. Stroke width is modulated by per-point pressure so firmer presses
//! draw thicker segments.

use ars_core::{RasterError, RasterImage, RasterPoint, RasterSpec, SignatureRasterizer};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use {
    ars_core::RasterFormat,
    wasm_bindgen::{JsCast, JsValue},
    web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, Window},
};

/// Production [`SignatureRasterizer`] for browser builds.
///
/// Rasterizes strokes via an offscreen `<canvas>` 2D context and `toDataURL`.
/// On non-wasm targets (e.g. SSR type-checking) every call returns
/// [`RasterError::Unsupported`].
#[derive(Debug, Default, Clone, Copy)]
pub struct WebSignatureRasterizer;

impl SignatureRasterizer for WebSignatureRasterizer {
    fn rasterize(
        &self,
        strokes: &[Vec<RasterPoint>],
        spec: &RasterSpec,
    ) -> Result<RasterImage, RasterError> {
        rasterize_impl(strokes, spec)
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn rasterize_impl(
    strokes: &[Vec<RasterPoint>],
    spec: &RasterSpec,
) -> Result<RasterImage, RasterError> {
    // A zero-sized canvas makes `toDataURL` return the `data:,` sentinel rather
    // than an encoded image, which would otherwise decode to an empty byte
    // buffer that callers could mistake for a valid export. Reject it up front.
    if spec.width == 0 || spec.height == 0 {
        return Err(RasterError::Backend(
            "raster spec has zero width or height".into(),
        ));
    }

    let window = web_sys::window().ok_or_else(|| RasterError::Backend("no window".into()))?;

    let document = window
        .document()
        .ok_or_else(|| RasterError::Backend("no document".into()))?;

    let canvas: HtmlCanvasElement = document
        .create_element("canvas")
        .map_err(|_| RasterError::Backend("create_element(canvas) failed".into()))?
        .dyn_into()
        .map_err(|_| RasterError::Backend("element is not a canvas".into()))?;

    canvas.set_width(spec.width);
    canvas.set_height(spec.height);

    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .map_err(|_| RasterError::Backend("get_context(2d) failed".into()))?
        .ok_or_else(|| RasterError::Backend("no 2d context".into()))?
        .dyn_into()
        .map_err(|_| RasterError::Backend("context is not 2d".into()))?;

    if let Some(background) = &spec.background {
        ctx.set_fill_style_str(background);
        ctx.fill_rect(0.0, 0.0, f64::from(spec.width), f64::from(spec.height));
    }

    ctx.set_stroke_style_str(&spec.pen_color);
    ctx.set_line_cap("round");
    ctx.set_line_join("round");

    for stroke in strokes {
        for segment in stroke.windows(2) {
            let (from, to) = (segment[0], segment[1]);

            ctx.set_line_width(spec.pen_width * pressure_scale(from.pressure, to.pressure));
            ctx.begin_path();
            ctx.move_to(from.x, from.y);
            ctx.line_to(to.x, to.y);
            ctx.stroke();
        }
    }

    let mime = spec.format.mime();

    let data_url = match spec.format {
        // PNG is lossless and ignores quality.
        RasterFormat::Png => canvas.to_data_url_with_type(mime),

        // JPEG and WebP honor the quality factor.
        RasterFormat::Jpeg | RasterFormat::Webp => {
            canvas.to_data_url_with_type_and_encoder_options(mime, &JsValue::from_f64(spec.quality))
        }
    }
    .map_err(|_| RasterError::Backend("toDataURL failed".into()))?;

    // A browser that cannot encode the requested type (e.g. Safari for WebP)
    // silently substitutes PNG, so trust the data URL's own MIME rather than the
    // request, falling back to the requested format only if it is unparseable.
    let (actual_format, bytes) = decode_data_url(&window, &data_url, spec.format)?;

    Ok(RasterImage {
        format: actual_format,
        bytes,
    })
}

/// Maps the pressures at a segment's two endpoints to a stroke-width multiplier
/// in `0.5..=1.0`, so a segment is always visible (never zero width) while
/// firmer presses render thicker.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn pressure_scale(from: f64, to: f64) -> f64 {
    let average = ((from + to) / 2.0).clamp(0.0, 1.0);

    0.5 + 0.5 * average
}

/// Decodes a `data:<mime>;base64,<payload>` URL into the MIME-derived format and
/// raw bytes via `window.atob`.
///
/// The returned format comes from the data URL's own MIME (so a browser's silent
/// substitution is reported truthfully), falling back to `requested` if the
/// header is unparseable. `atob` yields a binary string whose code points are
/// byte values `0..=255`; truncating each `char` to `u8` recovers the bytes.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn decode_data_url(
    window: &Window,
    data_url: &str,
    requested: RasterFormat,
) -> Result<(RasterFormat, Vec<u8>), RasterError> {
    let (header, base64) = data_url
        .split_once(',')
        .ok_or_else(|| RasterError::Backend("malformed data URL".into()))?;

    // header looks like `data:image/png;base64`
    let format = header
        .strip_prefix("data:")
        .and_then(|rest| rest.split(';').next())
        .and_then(RasterFormat::from_mime)
        .unwrap_or(requested);

    let binary = window
        .atob(base64)
        .map_err(|_| RasterError::Backend("atob failed".into()))?;

    Ok((format, binary.chars().map(|code| code as u8).collect()))
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn rasterize_impl(
    _strokes: &[Vec<RasterPoint>],
    _spec: &RasterSpec,
) -> Result<RasterImage, RasterError> {
    Err(RasterError::Unsupported)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod native_tests {
    use super::*;

    #[test]
    fn rasterize_is_unsupported_off_wasm() {
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

        assert_eq!(
            WebSignatureRasterizer.rasterize(&strokes, &RasterSpec::new(10, 10)),
            Err(RasterError::Unsupported)
        );
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use ars_core::{RasterFormat, RasterPoint, RasterSpec, SignatureRasterizer};
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn sample_strokes() -> Vec<Vec<RasterPoint>> {
        vec![vec![
            RasterPoint {
                x: 5.0,
                y: 5.0,
                pressure: 0.4,
            },
            RasterPoint {
                x: 50.0,
                y: 50.0,
                pressure: 0.9,
            },
            RasterPoint {
                x: 90.0,
                y: 10.0,
                pressure: 0.2,
            },
        ]]
    }

    #[wasm_bindgen_test]
    fn rasterizes_png_with_magic_bytes() {
        let image = WebSignatureRasterizer
            .rasterize(&sample_strokes(), &RasterSpec::new(100, 100))
            .expect("png rasterization succeeds in the browser");

        assert_eq!(image.format, RasterFormat::Png);
        assert!(
            image.bytes.starts_with(&[0x89, b'P', b'N', b'G']),
            "expected PNG magic, got {:?}",
            &image.bytes.get(..8)
        );
    }

    #[wasm_bindgen_test]
    fn rasterizes_jpeg_with_magic_bytes() {
        let spec = RasterSpec::new(100, 100)
            .format(RasterFormat::Jpeg)
            .background("#ffffff")
            .quality(0.8);

        let image = WebSignatureRasterizer
            .rasterize(&sample_strokes(), &spec)
            .expect("jpeg rasterization succeeds in the browser");

        assert_eq!(image.format, RasterFormat::Jpeg);
        assert!(
            image.bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
            "expected JPEG magic, got {:?}",
            &image.bytes.get(..3)
        );
    }

    #[wasm_bindgen_test]
    fn webp_request_reports_the_format_it_actually_produced() {
        let spec = RasterSpec::new(100, 100)
            .format(RasterFormat::Webp)
            .quality(0.8);

        let image = WebSignatureRasterizer
            .rasterize(&sample_strokes(), &spec)
            .expect("webp-capable browsers encode webp; others fall back to png");

        // The reported format must match the actual bytes — never a webp label
        // on png bytes when the browser silently fell back.
        match image.format {
            RasterFormat::Webp => {
                assert!(image.bytes.len() >= 12, "webp too short");
                assert_eq!(&image.bytes[0..4], b"RIFF");
                assert_eq!(&image.bytes[8..12], b"WEBP");
            }

            RasterFormat::Png => {
                assert!(image.bytes.starts_with(&[0x89, b'P', b'N', b'G']));
            }

            RasterFormat::Jpeg => panic!("webp request must never report jpeg"),
        }
    }

    #[wasm_bindgen_test]
    fn zero_sized_spec_is_rejected_not_empty_bytes() {
        // A 0-dimension canvas would make toDataURL return the `data:,` sentinel,
        // which must surface as an error rather than empty "valid" image bytes.
        let result = WebSignatureRasterizer.rasterize(&sample_strokes(), &RasterSpec::new(0, 100));
        assert!(matches!(result, Err(RasterError::Backend(_))));

        let result = WebSignatureRasterizer.rasterize(&sample_strokes(), &RasterSpec::new(100, 0));
        assert!(matches!(result, Err(RasterError::Backend(_))));
    }

    #[wasm_bindgen_test]
    fn empty_signature_still_encodes_a_blank_image() {
        let image = WebSignatureRasterizer
            .rasterize(&[], &RasterSpec::new(20, 20))
            .expect("blank canvas still encodes");

        assert!(image.bytes.starts_with(&[0x89, b'P', b'N', b'G']));
    }
}
