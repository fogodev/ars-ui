//! Overflow math and clipping-boundary helpers for floating positioning.

use super::types::{Axis, Boundary, Overflow, Placement, Rect};

/// Resolves the clipping boundary rect for positioning calculations.
///
/// Non-web targets cannot measure DOM elements for [`Boundary::Element`], so
/// those cases fall back to the provided viewport rect.
#[must_use]
pub(crate) fn resolve_boundary_rect(boundary: &Boundary, viewport: &Rect) -> Rect {
    match boundary {
        Boundary::Viewport => *viewport,
        Boundary::Element(element) => resolve_element_boundary_rect(element).unwrap_or(*viewport),
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn resolve_element_boundary_rect(element: &super::types::ElementRef) -> Option<Rect> {
    let element = element.as_ref().downcast_ref::<web_sys::Element>()?;
    let rect = element.get_bounding_client_rect();
    Some(Rect {
        x: rect.x(),
        y: rect.y(),
        width: rect.width(),
        height: rect.height(),
    })
}

#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
fn resolve_element_boundary_rect(_element: &super::types::ElementRef) -> Option<Rect> {
    None
}

/// Returns `rect` inset equally on all sides by `padding`.
#[must_use]
pub(crate) fn inset_rect(rect: Rect, padding: f64) -> Rect {
    if padding <= 0.0 {
        return rect;
    }

    // Padding shrinks the usable clipping rect from all four sides at once.
    // The origin moves inward by `padding`, and width/height lose `2 * padding`
    // because each dimension is reduced on both edges. Clamp at zero so an
    // oversized padding value collapses the rect instead of producing negatives.
    let width = (rect.width - (padding * 2.0)).max(0.0);
    let height = (rect.height - (padding * 2.0)).max(0.0);

    Rect {
        x: rect.x + padding,
        y: rect.y + padding,
        width,
        height,
    }
}

/// Computes overflow on each side relative to the clipping boundary.
#[must_use]
pub(crate) fn detect_overflow(x: f64, y: f64, floating: &Rect, boundary: &Rect) -> Overflow {
    // Positive values mean the floating rect extends past that boundary edge;
    // negative values mean there is still room remaining on that side.
    Overflow {
        top: boundary.y - y,
        right: (x + floating.width) - boundary.right(),
        bottom: (y + floating.height) - boundary.bottom(),
        left: boundary.x - x,
    }
}

/// Returns `true` when the placement overflows on its main axis side.
#[must_use]
pub(crate) fn should_flip(overflow: &Overflow, placement: Placement) -> bool {
    match placement.side() {
        super::types::Side::Top => overflow.top > 0.0,
        super::types::Side::Right => overflow.right > 0.0,
        super::types::Side::Bottom => overflow.bottom > 0.0,
        super::types::Side::Left => overflow.left > 0.0,
    }
}

/// Sums the positive overflow on all sides.
#[must_use]
pub(crate) fn total_overflow(overflow: &Overflow) -> f64 {
    overflow.top.max(0.0)
        + overflow.right.max(0.0)
        + overflow.bottom.max(0.0)
        + overflow.left.max(0.0)
}

/// Computes the cross-axis shift required to keep the floating rect in bounds.
#[must_use]
pub(crate) fn compute_shift(
    x: f64,
    y: f64,
    floating: &Rect,
    boundary: &Rect,
    placement: Placement,
    shift_padding: f64,
) -> (f64, f64) {
    let shift_boundary = inset_rect(*boundary, shift_padding);

    match placement.main_axis() {
        Axis::Vertical => {
            // Top/Bottom placements may only slide horizontally; moving y would
            // change which side of the anchor the floating element is on.
            let clamped_x =
                clamp_coordinate(x, shift_boundary.x, shift_boundary.right() - floating.width);
            (clamped_x - x, 0.0)
        }
        Axis::Horizontal => {
            // Left/Right placements mirror the same rule on the vertical axis.
            let clamped_y = clamp_coordinate(
                y,
                shift_boundary.y,
                shift_boundary.bottom() - floating.height,
            );
            (0.0, clamped_y - y)
        }
    }
}

#[must_use]
fn clamp_coordinate(value: f64, min: f64, max: f64) -> f64 {
    if max < min {
        // When the padded boundary is smaller than the floating rect, "fit"
        // degenerates into pinning the coordinate to the padded start edge.
        min
    } else {
        value.clamp(min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inset_rect_reduces_size_and_moves_origin() {
        let rect = Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };

        assert_eq!(
            inset_rect(rect, 5.0),
            Rect {
                x: 15.0,
                y: 25.0,
                width: 90.0,
                height: 40.0,
            }
        );
    }

    #[test]
    fn inset_rect_clamps_to_zero_when_padding_is_too_large() {
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            width: 8.0,
            height: 6.0,
        };

        assert_eq!(
            inset_rect(rect, 10.0),
            Rect {
                x: 10.0,
                y: 10.0,
                width: 0.0,
                height: 0.0,
            }
        );
    }

    #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
    #[test]
    fn boundary_element_falls_back_to_viewport_outside_web_dom_targets() {
        #[cfg(target_arch = "wasm32")]
        let boundary = Boundary::Element(std::rc::Rc::new(()));
        #[cfg(not(target_arch = "wasm32"))]
        let boundary = Boundary::Element(std::sync::Arc::new(()));

        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 200.0,
        };

        assert_eq!(resolve_boundary_rect(&boundary, &viewport), viewport);
    }

    #[test]
    fn should_flip_checks_main_axis_overflow_for_all_side_variants() {
        let overflow = Overflow {
            top: 3.0,
            right: 4.0,
            bottom: 5.0,
            left: 6.0,
        };

        assert!(should_flip(&overflow, Placement::Top));
        assert!(should_flip(&overflow, Placement::Right));
        assert!(should_flip(&overflow, Placement::Bottom));
        assert!(should_flip(&overflow, Placement::Left));
    }

    #[test]
    fn compute_shift_clamps_to_min_when_padded_boundary_is_smaller_than_floating_rect() {
        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 80.0,
            height: 30.0,
        };
        let boundary = Rect {
            x: 0.0,
            y: 0.0,
            width: 40.0,
            height: 30.0,
        };

        assert_eq!(
            compute_shift(10.0, 5.0, &floating, &boundary, Placement::Bottom, 5.0),
            (-5.0, 0.0)
        );
    }
}
