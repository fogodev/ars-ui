//! Viewport measurement helpers for DOM positioning.
//!
//! These helpers provide a browser-backed way to measure the effective
//! viewport, preferring the visual viewport when available so floating
//! elements can stay within the user-visible region on mobile browsers.

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
use super::Rect;

/// Snapshot of the browser visual viewport used by fallback resolution.
#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
#[derive(Clone, Copy, Debug, PartialEq)]
struct VisualViewportMetrics {
    width: f64,
    height: f64,
    offset_left: f64,
    offset_top: f64,
}

/// Resolve the effective viewport rect from optional browser measurements.
#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
#[must_use]
fn resolve_viewport_rect(
    visual_viewport: Option<VisualViewportMetrics>,
    inner_width: Option<f64>,
    inner_height: Option<f64>,
) -> Rect {
    if let Some(visual_viewport) = visual_viewport {
        Rect {
            x: visual_viewport.offset_left,
            y: visual_viewport.offset_top,
            width: visual_viewport.width,
            height: visual_viewport.height,
        }
    } else {
        Rect {
            x: 0.0,
            y: 0.0,
            width: inner_width.unwrap_or(0.0),
            height: inner_height.unwrap_or(0.0),
        }
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn warn_on_zero_dimension(name: &str, value: f64) {
    #[cfg(feature = "debug")]
    if value == 0.0 {
        log::warn!("[ars-dom] {name} returned 0.0; window may not be fully initialized");
    }

    #[cfg(not(feature = "debug"))]
    let _ = (name, value);
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn inner_width(window: &web_sys::Window) -> Option<f64> {
    window.inner_width().ok().and_then(|value| value.as_f64())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn inner_height(window: &web_sys::Window) -> Option<f64> {
    window.inner_height().ok().and_then(|value| value.as_f64())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn visual_viewport_metrics(window: &web_sys::Window) -> Option<VisualViewportMetrics> {
    window
        .visual_viewport()
        .map(|viewport| VisualViewportMetrics {
            width: viewport.width(),
            height: viewport.height(),
            offset_left: viewport.offset_left(),
            offset_top: viewport.offset_top(),
        })
}

/// Return the current effective viewport width for positioning calculations.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
pub fn viewport_width(window: &web_sys::Window) -> f64 {
    let width =
        resolve_viewport_rect(visual_viewport_metrics(window), inner_width(window), None).width;

    warn_on_zero_dimension("viewport_width()", width);

    width
}

/// Return the current effective viewport height for positioning calculations.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
pub fn viewport_height(window: &web_sys::Window) -> f64 {
    let height =
        resolve_viewport_rect(visual_viewport_metrics(window), None, inner_height(window)).height;

    warn_on_zero_dimension("viewport_height()", height);

    height
}

/// Return the effective viewport rect for positioning calculations.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
pub fn viewport_rect(window: &web_sys::Window) -> Rect {
    let rect = resolve_viewport_rect(
        visual_viewport_metrics(window),
        inner_width(window),
        inner_height(window),
    );

    warn_on_zero_dimension("viewport_width()", rect.width);
    warn_on_zero_dimension("viewport_height()", rect.height);

    rect
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_returns_zero_rect_when_browser_values_are_absent() {
        assert_eq!(
            resolve_viewport_rect(None, None, None),
            Rect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            }
        );
    }

    #[test]
    fn visual_viewport_dimensions_override_inner_window_dimensions() {
        let rect = resolve_viewport_rect(
            Some(VisualViewportMetrics {
                width: 320.0,
                height: 240.0,
                offset_left: 12.0,
                offset_top: 18.0,
            }),
            Some(1024.0),
            Some(768.0),
        );

        assert_eq!(rect.width, 320.0);
        assert_eq!(rect.height, 240.0);
    }

    #[test]
    fn viewport_rect_uses_visual_viewport_offsets() {
        let rect = resolve_viewport_rect(
            Some(VisualViewportMetrics {
                width: 300.0,
                height: 200.0,
                offset_left: 15.5,
                offset_top: 20.25,
            }),
            Some(1024.0),
            Some(768.0),
        );

        assert_eq!(rect.x, 15.5);
        assert_eq!(rect.y, 20.25);
    }

    #[test]
    fn zero_dimension_fallback_is_safe_and_deterministic() {
        let rect = resolve_viewport_rect(None, Some(0.0), Some(0.0));

        assert_eq!(
            rect,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            }
        );
    }

    #[test]
    fn inner_window_dimensions_are_used_when_visual_viewport_is_absent() {
        let rect = resolve_viewport_rect(None, Some(1440.0), Some(900.0));

        assert_eq!(
            rect,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 1440.0,
                height: 900.0,
            }
        );
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn viewport_dimension_helpers_run_in_browser() {
        let window = web_sys::window().expect("window should exist in browser tests");

        let _ = viewport_width(&window);

        let _ = viewport_height(&window);
    }

    #[wasm_bindgen_test]
    fn warn_on_zero_dimension_is_safe_for_zero_values() {
        warn_on_zero_dimension("viewport_width()", 0.0);
        warn_on_zero_dimension("viewport_height()", 0.0);
    }

    #[wasm_bindgen_test]
    fn resolve_viewport_rect_prefers_visual_viewport_metrics_in_wasm_builds() {
        let rect = resolve_viewport_rect(
            Some(VisualViewportMetrics {
                width: 320.0,
                height: 240.0,
                offset_left: 12.0,
                offset_top: 18.0,
            }),
            Some(1024.0),
            Some(768.0),
        );

        assert_eq!(
            rect,
            Rect {
                x: 12.0,
                y: 18.0,
                width: 320.0,
                height: 240.0,
            }
        );
    }

    #[wasm_bindgen_test]
    fn resolve_viewport_rect_falls_back_without_visual_viewport_metrics_in_wasm_builds() {
        let rect = resolve_viewport_rect(None, Some(640.0), Some(480.0));

        assert_eq!(
            rect,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 640.0,
                height: 480.0,
            }
        );
    }

    #[wasm_bindgen_test]
    fn viewport_rect_matches_visual_viewport_when_available() {
        let window = web_sys::window().expect("window should exist in browser tests");

        let Some(visual_viewport) = window.visual_viewport() else {
            return;
        };

        let rect = viewport_rect(&window);

        assert_eq!(rect.width, visual_viewport.width());
        assert_eq!(rect.height, visual_viewport.height());
        assert_eq!(rect.x, visual_viewport.offset_left());
        assert_eq!(rect.y, visual_viewport.offset_top());
    }

    #[wasm_bindgen_test]
    fn viewport_rect_falls_back_to_inner_window_dimensions_when_visual_viewport_is_absent() {
        let window = web_sys::window().expect("window should exist in browser tests");

        if window.visual_viewport().is_some() {
            return;
        }

        let rect = viewport_rect(&window);

        let width = window
            .inner_width()
            .expect("innerWidth should be readable")
            .as_f64()
            .expect("innerWidth should be numeric");

        let height = window
            .inner_height()
            .expect("innerHeight should be readable")
            .as_f64()
            .expect("innerHeight should be numeric");

        assert_eq!(rect.width, width);
        assert_eq!(rect.height, height);
        assert_eq!(rect.x, 0.0);
        assert_eq!(rect.y, 0.0);
    }
}
