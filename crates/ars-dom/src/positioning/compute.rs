//! Core floating-position computation for overlays.

use ars_i18n::ResolvedDirection;

use super::{
    overflow::{
        compute_shift, detect_overflow, inset_rect, resolve_boundary_rect, should_flip,
        total_overflow,
    },
    types::{Alignment, Offset, Placement, PositioningOptions, PositioningResult, Rect, Side},
};

const ALL_PHYSICAL_PLACEMENTS: [Placement; 12] = [
    Placement::Top,
    Placement::TopStart,
    Placement::TopEnd,
    Placement::Bottom,
    Placement::BottomStart,
    Placement::BottomEnd,
    Placement::Left,
    Placement::LeftStart,
    Placement::LeftEnd,
    Placement::Right,
    Placement::RightStart,
    Placement::RightEnd,
];

const CENTER_AUTO_PLACEMENTS: [Placement; 4] = [
    Placement::Top,
    Placement::Bottom,
    Placement::Left,
    Placement::Right,
];

const START_AUTO_PLACEMENTS: [Placement; 4] = [
    Placement::TopStart,
    Placement::BottomStart,
    Placement::LeftStart,
    Placement::RightStart,
];

const END_AUTO_PLACEMENTS: [Placement; 4] = [
    Placement::TopEnd,
    Placement::BottomEnd,
    Placement::LeftEnd,
    Placement::RightEnd,
];

/// Calculates the position of a floating element relative to an anchor rect.
///
/// The algorithm resolves logical placements, applies offsets, flips or shifts
/// to avoid overflow, computes optional arrow placement, and returns max-size
/// constraints for the chosen side.
#[must_use]
pub fn compute_position(
    anchor: &Rect,
    floating: &Rect,
    viewport: &Rect,
    options: &PositioningOptions,
) -> PositioningResult {
    // Overflow, shift, and max-size all operate against the effective clipping
    // boundary, not the raw viewport. Boundary::Element is resolved up front and
    // boundary padding shrinks that rect once so every later step shares it.
    let boundary = inset_rect(
        resolve_boundary_rect(&options.boundary, viewport),
        options.boundary_padding,
    );
    let auto_requested = is_auto_placement_enabled(options);

    // Placement selection happens in two phases:
    // 1. choose an initial side/alignment, either directly or by auto-scoring
    // 2. optionally try flip candidates only when placement was not already
    //    chosen by auto placement
    //
    // That separation keeps auto placement as the "pick the best pose up front"
    // path, while flip acts as a recovery step for an explicitly requested side.
    let mut actual_placement = if auto_requested {
        compute_auto_placement(anchor, floating, &boundary, options)
    } else {
        options.placement.resolve_logical(options.dir)
    };

    // Start from the raw side/alignment coordinates, then project the logical
    // main/cross-axis offsets into viewport x/y deltas. Every later candidate
    // comparison uses this same helper path so scoring is consistent.
    let (mut x, mut y) = compute_coords(anchor, floating, actual_placement, options.overlap);
    let (offset_x, offset_y) = resolve_offset(options.offset, actual_placement);
    x += offset_x;
    y += offset_y;

    if !auto_requested && options.flip {
        let overflow = detect_overflow(x, y, floating, &boundary);
        if should_flip(&overflow, actual_placement) {
            // The requested placement remains the baseline. Each fallback is
            // fully recomputed with overlap + offset applied, then compared by
            // total clipped overflow. This avoids subtle bias from reusing
            // partially adjusted coordinates from the previous candidate.
            let mut best_total = total_overflow(&overflow);
            let mut best_x = x;
            let mut best_y = y;
            let mut best_placement = actual_placement;

            for candidate in flip_candidates(options, actual_placement) {
                let (candidate_x, candidate_y) =
                    positioned_coords(anchor, floating, candidate, options.overlap, options.offset);
                let candidate_overflow =
                    detect_overflow(candidate_x, candidate_y, floating, &boundary);
                let candidate_total = total_overflow(&candidate_overflow);

                // Lower total overflow means less of the floating rect sits
                // outside the padded clipping boundary, regardless of which
                // individual edge is responsible for that overflow.
                if candidate_total < best_total {
                    best_total = candidate_total;
                    best_x = candidate_x;
                    best_y = candidate_y;
                    best_placement = candidate;
                }

                // A zero-overflow candidate cannot be improved on, so the scan
                // can stop without examining the rest of the fallback list.
                if candidate_total == 0.0 {
                    break;
                }
            }

            x = best_x;
            y = best_y;
            actual_placement = best_placement;
        }
    }

    if options.shift {
        // Shift runs after the final side has been chosen. It only nudges along
        // the cross axis for that side, preserving the semantic placement while
        // pulling the rect back inside the padded clipping boundary.
        let (shift_x, shift_y) = compute_shift(
            x,
            y,
            floating,
            &boundary,
            actual_placement,
            options.shift_padding,
        );
        x += shift_x;
        y += shift_y;
    }

    let (arrow_x, arrow_y) = if options.arrow_padding > 0.0 {
        compute_arrow_position(
            anchor,
            x,
            y,
            floating,
            actual_placement,
            options.arrow_padding,
        )
    } else {
        (None, None)
    };

    let (max_width, max_height) = if options.auto_max_size {
        compute_max_size(x, y, floating, &boundary, actual_placement)
    } else {
        (None, None)
    };

    PositioningResult {
        x,
        y,
        actual_placement,
        arrow_x,
        arrow_y,
        max_height,
        max_width,
    }
}

#[must_use]
fn is_auto_placement_enabled(options: &PositioningOptions) -> bool {
    options.auto_placement
        || matches!(
            options.placement,
            Placement::Auto | Placement::AutoStart | Placement::AutoEnd
        )
}

#[must_use]
fn positioned_coords(
    anchor: &Rect,
    floating: &Rect,
    placement: Placement,
    overlap: bool,
    offset: Offset,
) -> (f64, f64) {
    // This helper is intentionally the single "full pose" computation used by
    // auto placement and flip scoring. Keeping that logic in one place ensures
    // every candidate is compared after the same overlap and offset rules.
    let (mut x, mut y) = compute_coords(anchor, floating, placement, overlap);
    let (offset_x, offset_y) = resolve_offset(offset, placement);
    x += offset_x;
    y += offset_y;
    (x, y)
}

#[must_use]
fn compute_auto_placement(
    anchor: &Rect,
    floating: &Rect,
    boundary: &Rect,
    options: &PositioningOptions,
) -> Placement {
    // Auto placement is a pure scoring pass: compute each candidate with the
    // same overlap/offset rules as normal placement, then keep the one with the
    // least total overflow. A perfect fit short-circuits the search.
    let candidates = auto_candidates(options);
    let mut best_placement = candidates[0];
    let mut best_overflow = f64::MAX;

    for &candidate in candidates {
        let (x, y) =
            positioned_coords(anchor, floating, candidate, options.overlap, options.offset);
        let overflow = detect_overflow(x, y, floating, boundary);
        let total = total_overflow(&overflow);

        if total < best_overflow {
            best_overflow = total;
            best_placement = candidate;
        }

        if total == 0.0 {
            break;
        }
    }

    best_placement
}

#[must_use]
fn auto_candidates(options: &PositioningOptions) -> &'static [Placement] {
    if options.auto_placement {
        // Boolean auto placement means "consider every physical placement",
        // whereas Placement::Auto* restricts the search to one alignment family.
        &ALL_PHYSICAL_PLACEMENTS
    } else {
        match options.placement {
            Placement::Auto => &CENTER_AUTO_PLACEMENTS,
            Placement::AutoStart => &START_AUTO_PLACEMENTS,
            Placement::AutoEnd => &END_AUTO_PLACEMENTS,
            _ => &ALL_PHYSICAL_PLACEMENTS,
        }
    }
}

fn flip_candidates(options: &PositioningOptions, current: Placement) -> Vec<Placement> {
    let mut candidates = Vec::new();

    // Explicit fallbacks are considered first and may themselves expand from
    // logical or auto placements into concrete physical candidates.
    for placement in &options.fallback_placements {
        extend_unique(
            &mut candidates,
            expanded_placements(*placement, options.dir),
        );
    }

    // The direct opposite is always appended as a last recovery option, even
    // when explicit fallbacks exist, so a clearly better mirrored placement is
    // still available if the fallback list was too narrow.
    extend_unique(
        &mut candidates,
        expanded_placements(current.opposite(), options.dir),
    );
    candidates.retain(|candidate| *candidate != current);
    candidates
}

fn extend_unique(candidates: &mut Vec<Placement>, incoming: Vec<Placement>) {
    for placement in incoming {
        if !candidates.contains(&placement) {
            candidates.push(placement);
        }
    }
}

#[must_use]
fn expanded_placements(placement: Placement, dir: ResolvedDirection) -> Vec<Placement> {
    // Flip scoring only works on concrete physical placements. Auto variants
    // therefore expand into the exact family they would have searched during
    // initial auto placement, after logical start/end have been resolved.
    match placement.resolve_logical(dir) {
        Placement::Auto => CENTER_AUTO_PLACEMENTS.to_vec(),
        Placement::AutoStart => START_AUTO_PLACEMENTS.to_vec(),
        Placement::AutoEnd => END_AUTO_PLACEMENTS.to_vec(),
        resolved => vec![resolved],
    }
}

#[must_use]
fn compute_coords(
    anchor: &Rect,
    floating: &Rect,
    placement: Placement,
    overlap: bool,
) -> (f64, f64) {
    let (side, alignment) = placement.side_and_alignment();

    // Top/Bottom placements choose x from cross-axis alignment. Left/Right use
    // the same alignment logic for y. The main-axis coordinate then decides
    // whether the floating rect sits outside the anchor or shares that edge
    // when overlap mode is enabled.
    let x = match side {
        Side::Top | Side::Bottom => match alignment {
            Alignment::Start => anchor.x,
            Alignment::Center => anchor.center_x() - (floating.width / 2.0),
            Alignment::End => anchor.right() - floating.width,
        },
        Side::Left => {
            if overlap {
                anchor.x
            } else {
                anchor.x - floating.width
            }
        }
        Side::Right => {
            if overlap {
                anchor.right() - floating.width
            } else {
                anchor.right()
            }
        }
    };

    // The y computation mirrors the same decomposition: vertical sides choose
    // their main-axis attachment here, while horizontal sides reuse alignment
    // math to decide where the floating rect sits along the anchor's height.
    let y = match side {
        Side::Top => {
            if overlap {
                anchor.y
            } else {
                anchor.y - floating.height
            }
        }
        Side::Bottom => {
            if overlap {
                anchor.bottom() - floating.height
            } else {
                anchor.bottom()
            }
        }
        Side::Left | Side::Right => match alignment {
            Alignment::Start => anchor.y,
            Alignment::Center => anchor.center_y() - (floating.height / 2.0),
            Alignment::End => anchor.bottom() - floating.height,
        },
    };

    (x, y)
}

#[must_use]
fn resolve_offset(offset: Offset, placement: Placement) -> (f64, f64) {
    // main_axis always means "away from the anchor on the chosen side".
    // cross_axis always means "slide along the aligned edge". The signs here
    // convert those logical directions into viewport-space x/y deltas.
    match placement.side() {
        Side::Top => (offset.cross_axis, -offset.main_axis),
        Side::Bottom => (offset.cross_axis, offset.main_axis),
        Side::Left => (-offset.main_axis, offset.cross_axis),
        Side::Right => (offset.main_axis, offset.cross_axis),
    }
}

#[must_use]
fn compute_arrow_position(
    anchor: &Rect,
    x: f64,
    y: f64,
    floating: &Rect,
    placement: Placement,
    arrow_padding: f64,
) -> (Option<f64>, Option<f64>) {
    let (side, _) = placement.side_and_alignment();

    // Arrow coordinates are returned in floating-local space. Subtracting the
    // floating rect's top-left corner converts the anchor center from viewport
    // coordinates into a distance measured along the floating edge itself.
    match side {
        Side::Top | Side::Bottom => {
            // The arrow stays centered on the anchor in floating-local space,
            // then clamps to the usable edge span so it never escapes the box.
            let center = anchor.center_x() - x;
            // TODO: Extend arrow clamping to account for border radius and arrow size
            // once those style metrics are available to the pure-geometry layer.
            let min = arrow_padding;
            let max = (floating.width - arrow_padding).max(min);
            (Some(center.clamp(min, max)), None)
        }
        Side::Left | Side::Right => {
            // Horizontal placements invert the same idea onto the y axis.
            let center = anchor.center_y() - y;
            // TODO: Extend arrow clamping to account for border radius and arrow size
            // once those style metrics are available to the pure-geometry layer.
            let min = arrow_padding;
            let max = (floating.height - arrow_padding).max(min);
            (None, Some(center.clamp(min, max)))
        }
    }
}

#[must_use]
fn compute_max_size(
    x: f64,
    y: f64,
    floating: &Rect,
    boundary: &Rect,
    placement: Placement,
) -> (Option<f64>, Option<f64>) {
    // Max size is asymmetric by design. The main axis only has the remaining
    // room on the chosen side, while the cross axis may span the full clipped
    // boundary because shift already keeps the floating rect inside that span.
    let max_width = match placement.side() {
        // For left placement, x is the floating rect's left edge. Adding the
        // current width yields the anchored edge nearest the boundary's rightward
        // direction, so subtracting boundary.x gives the remaining usable width.
        Side::Left => (x + floating.width - boundary.x).max(0.0),
        // For right placement, x already starts at the boundary-facing edge, so
        // the remaining room is just the distance to the clipping boundary's end.
        Side::Right => (boundary.right() - x).max(0.0),
        Side::Top | Side::Bottom => boundary.width.max(0.0),
    };

    let max_height = match placement.side() {
        // Top placement mirrors the left-placement reasoning on the vertical axis.
        Side::Top => (y + floating.height - boundary.y).max(0.0),
        // Bottom placement mirrors right-placement reasoning on the vertical axis.
        Side::Bottom => (boundary.bottom() - y).max(0.0),
        Side::Left | Side::Right => boundary.height.max(0.0),
    };

    (Some(max_width), Some(max_height))
}

#[cfg(test)]
mod tests {
    use super::{
        super::types::{Boundary, PositioningOptions},
        *,
    };

    fn anchor() -> Rect {
        Rect {
            x: 100.0,
            y: 100.0,
            width: 50.0,
            height: 30.0,
        }
    }

    fn floating() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 40.0,
            height: 20.0,
        }
    }

    fn viewport() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 300.0,
            height: 240.0,
        }
    }

    fn base_options(placement: Placement) -> PositioningOptions {
        PositioningOptions {
            placement,
            flip: false,
            shift: false,
            auto_max_size: false,
            boundary_padding: 0.0,
            shift_padding: 0.0,
            arrow_padding: 0.0,
            ..PositioningOptions::default()
        }
    }

    #[test]
    fn basic_bottom_center_position_is_computed() {
        let result = compute_position(
            &anchor(),
            &floating(),
            &viewport(),
            &base_options(Placement::Bottom),
        );
        assert_eq!(result.x, 105.0);
        assert_eq!(result.y, 130.0);
        assert_eq!(result.actual_placement, Placement::Bottom);
    }

    #[test]
    fn basic_top_start_position_is_computed() {
        let result = compute_position(
            &anchor(),
            &floating(),
            &viewport(),
            &base_options(Placement::TopStart),
        );
        assert_eq!(result.x, 100.0);
        assert_eq!(result.y, 80.0);
        assert_eq!(result.actual_placement, Placement::TopStart);
    }

    #[test]
    fn basic_left_end_position_is_computed() {
        let result = compute_position(
            &anchor(),
            &floating(),
            &viewport(),
            &base_options(Placement::LeftEnd),
        );
        assert_eq!(result.x, 60.0);
        assert_eq!(result.y, 110.0);
        assert_eq!(result.actual_placement, Placement::LeftEnd);
    }

    #[test]
    fn basic_right_center_position_is_computed() {
        let result = compute_position(
            &anchor(),
            &floating(),
            &viewport(),
            &base_options(Placement::Right),
        );
        assert_eq!(result.x, 150.0);
        assert_eq!(result.y, 105.0);
        assert_eq!(result.actual_placement, Placement::Right);
    }

    #[test]
    fn vertical_offsets_apply_on_main_and_cross_axes() {
        let mut options = base_options(Placement::Bottom);
        options.offset = Offset {
            main_axis: 10.0,
            cross_axis: -5.0,
        };

        let result = compute_position(&anchor(), &floating(), &viewport(), &options);
        assert_eq!(result.x, 100.0);
        assert_eq!(result.y, 140.0);
    }

    #[test]
    fn horizontal_offsets_apply_on_main_and_cross_axes() {
        let mut options = base_options(Placement::Left);
        options.offset = Offset {
            main_axis: 12.0,
            cross_axis: -4.0,
        };

        let result = compute_position(&anchor(), &floating(), &viewport(), &options);
        assert_eq!(result.x, 48.0);
        assert_eq!(result.y, 101.0);
    }

    #[test]
    fn flip_prefers_opposite_side_when_primary_overflows() {
        let anchor = Rect {
            x: 100.0,
            y: 170.0,
            width: 40.0,
            height: 20.0,
        };
        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 60.0,
            height: 50.0,
        };
        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            width: 240.0,
            height: 200.0,
        };
        let mut options = base_options(Placement::Bottom);
        options.flip = true;

        let result = compute_position(&anchor, &floating, &viewport, &options);
        assert_eq!(result.actual_placement, Placement::Top);
        assert_eq!(result.x, 90.0);
        assert_eq!(result.y, 120.0);
    }

    #[test]
    fn fallback_placements_beat_direct_opposite_when_they_fit_better() {
        let anchor = Rect {
            x: 90.0,
            y: 90.0,
            width: 20.0,
            height: 20.0,
        };
        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 60.0,
            height: 40.0,
        };
        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            width: 120.0,
            height: 120.0,
        };
        let mut options = base_options(Placement::BottomStart);
        options.flip = true;
        options.fallback_placements = vec![Placement::LeftEnd];

        let result = compute_position(&anchor, &floating, &viewport, &options);
        assert_eq!(result.actual_placement, Placement::LeftEnd);
        assert_eq!(result.x, 30.0);
        assert_eq!(result.y, 70.0);
    }

    #[test]
    fn shift_clamps_vertical_placements_using_boundary_and_shift_padding() {
        let anchor = Rect {
            x: 160.0,
            y: 80.0,
            width: 30.0,
            height: 20.0,
        };
        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 60.0,
            height: 30.0,
        };
        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 180.0,
        };
        let mut options = base_options(Placement::Bottom);
        options.shift = true;
        options.boundary_padding = 10.0;
        options.shift_padding = 5.0;

        let result = compute_position(&anchor, &floating, &viewport, &options);
        assert_eq!(result.x, 125.0);
        assert_eq!(result.y, 100.0);
    }

    #[test]
    fn shift_clamps_horizontal_placements_using_boundary_and_shift_padding() {
        let anchor = Rect {
            x: 90.0,
            y: 180.0,
            width: 20.0,
            height: 20.0,
        };
        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 50.0,
            height: 40.0,
        };
        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            width: 220.0,
            height: 200.0,
        };
        let mut options = base_options(Placement::Right);
        options.shift = true;
        options.boundary_padding = 8.0;
        options.shift_padding = 4.0;

        let result = compute_position(&anchor, &floating, &viewport, &options);
        assert_eq!(result.x, 110.0);
        assert_eq!(result.y, 148.0);
    }

    #[test]
    fn arrow_position_is_computed_for_vertical_placements() {
        let mut options = base_options(Placement::Bottom);
        options.arrow_padding = 8.0;

        let result = compute_position(&anchor(), &floating(), &viewport(), &options);
        assert_eq!(result.arrow_x, Some(20.0));
        assert_eq!(result.arrow_y, None);
    }

    #[test]
    fn arrow_position_clamps_to_padding_near_edges() {
        let anchor = Rect {
            x: 0.0,
            y: 40.0,
            width: 10.0,
            height: 20.0,
        };
        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 40.0,
            height: 30.0,
        };
        let mut options = base_options(Placement::BottomStart);
        options.arrow_padding = 8.0;

        let result = compute_position(&anchor, &floating, &viewport(), &options);
        assert_eq!(result.arrow_x, Some(8.0));
        assert_eq!(result.arrow_y, None);
    }

    #[test]
    fn arrow_position_is_computed_for_horizontal_placements() {
        let mut options = base_options(Placement::Left);
        options.arrow_padding = 6.0;

        let result = compute_position(&anchor(), &floating(), &viewport(), &options);
        assert_eq!(result.arrow_x, None);
        assert_eq!(result.arrow_y, Some(10.0));
    }

    #[test]
    fn placement_auto_picks_the_best_centered_side() {
        let anchor = Rect {
            x: 120.0,
            y: 10.0,
            width: 40.0,
            height: 20.0,
        };
        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 80.0,
            height: 40.0,
        };
        let mut options = base_options(Placement::Auto);
        options.flip = true;

        let result = compute_position(&anchor, &floating, &viewport(), &options);
        assert_eq!(result.actual_placement, Placement::Bottom);
        assert_eq!(result.y, 30.0);
    }

    #[test]
    fn placement_auto_start_preserves_start_alignment() {
        let anchor = Rect {
            x: 10.0,
            y: 10.0,
            width: 30.0,
            height: 20.0,
        };
        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 60.0,
            height: 30.0,
        };
        let options = base_options(Placement::AutoStart);

        let result = compute_position(&anchor, &floating, &viewport(), &options);
        assert_eq!(result.actual_placement, Placement::BottomStart);
        assert_eq!(result.x, 10.0);
    }

    #[test]
    fn placement_auto_end_preserves_end_alignment() {
        let anchor = Rect {
            x: 240.0,
            y: 200.0,
            width: 30.0,
            height: 20.0,
        };
        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 60.0,
            height: 30.0,
        };
        let options = base_options(Placement::AutoEnd);

        let result = compute_position(&anchor, &floating, &viewport(), &options);
        assert_eq!(result.actual_placement, Placement::TopEnd);
        assert_eq!(result.x, 210.0);
    }

    #[test]
    fn boolean_auto_placement_considers_all_twelve_physical_placements() {
        let anchor = Rect {
            x: 280.0,
            y: 20.0,
            width: 20.0,
            height: 20.0,
        };
        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 70.0,
            height: 50.0,
        };
        let mut options = base_options(Placement::Bottom);
        options.auto_placement = true;

        let result = compute_position(&anchor, &floating, &viewport(), &options);
        assert_eq!(result.actual_placement, Placement::BottomEnd);
        assert_eq!(result.x, 230.0);
    }

    #[test]
    fn non_auto_placement_uses_physical_candidate_list_when_auto_is_disabled() {
        let options = base_options(Placement::Bottom);
        assert_eq!(auto_candidates(&options), &ALL_PHYSICAL_PLACEMENTS);
    }

    #[test]
    fn expanded_placements_resolve_auto_variants_into_expected_candidate_sets() {
        assert_eq!(
            expanded_placements(Placement::Auto, ResolvedDirection::Ltr),
            CENTER_AUTO_PLACEMENTS.to_vec()
        );
        assert_eq!(
            expanded_placements(Placement::AutoStart, ResolvedDirection::Ltr),
            START_AUTO_PLACEMENTS.to_vec()
        );
        assert_eq!(
            expanded_placements(Placement::AutoEnd, ResolvedDirection::Rtl),
            END_AUTO_PLACEMENTS.to_vec()
        );
    }

    #[test]
    fn flip_candidates_expand_auto_fallbacks_and_keep_opposite_as_recovery_option() {
        let mut options = base_options(Placement::Bottom);
        options.fallback_placements = vec![Placement::AutoStart];

        assert_eq!(
            flip_candidates(&options, Placement::Bottom),
            vec![
                Placement::TopStart,
                Placement::BottomStart,
                Placement::LeftStart,
                Placement::RightStart,
                Placement::Top,
            ]
        );
    }

    #[test]
    fn overlap_places_floating_on_the_anchor_edge_instead_of_outside_it() {
        let mut options = base_options(Placement::Bottom);
        options.overlap = true;

        let result = compute_position(&anchor(), &floating(), &viewport(), &options);
        assert_eq!(result.x, 105.0);
        assert_eq!(result.y, 110.0);
    }

    #[test]
    fn overlap_shares_anchor_edge_for_top_left_and_right_placements() {
        let anchor = anchor();
        let floating = floating();

        assert_eq!(
            compute_coords(&anchor, &floating, Placement::Top, true),
            (105.0, 100.0)
        );
        assert_eq!(
            compute_coords(&anchor, &floating, Placement::Left, true),
            (100.0, 105.0)
        );
        assert_eq!(
            compute_coords(&anchor, &floating, Placement::Right, true),
            (110.0, 105.0)
        );
    }

    #[test]
    fn logical_placements_are_resolved_before_positioning() {
        let mut options = base_options(Placement::StartTop);
        options.dir = ResolvedDirection::Rtl;

        let result = compute_position(&anchor(), &floating(), &viewport(), &options);
        assert_eq!(result.actual_placement, Placement::RightStart);
        assert_eq!(result.x, 150.0);
        assert_eq!(result.y, 100.0);
    }

    #[test]
    fn max_size_uses_remaining_space_for_vertical_placements() {
        let mut options = base_options(Placement::Bottom);
        options.auto_max_size = true;

        let result = compute_position(&anchor(), &floating(), &viewport(), &options);
        assert_eq!(result.max_width, Some(300.0));
        assert_eq!(result.max_height, Some(110.0));
    }

    #[test]
    fn max_size_uses_remaining_space_for_horizontal_placements() {
        let mut options = base_options(Placement::Left);
        options.auto_max_size = true;

        let result = compute_position(&anchor(), &floating(), &viewport(), &options);
        assert_eq!(result.max_width, Some(100.0));
        assert_eq!(result.max_height, Some(240.0));
    }

    #[test]
    fn max_size_uses_remaining_space_for_top_and_right_branches() {
        let mut top_options = base_options(Placement::Top);
        top_options.auto_max_size = true;
        let top = compute_position(&anchor(), &floating(), &viewport(), &top_options);
        assert_eq!(top.max_width, Some(300.0));
        assert_eq!(top.max_height, Some(100.0));

        let mut right_options = base_options(Placement::Right);
        right_options.auto_max_size = true;
        let right = compute_position(&anchor(), &floating(), &viewport(), &right_options);
        assert_eq!(right.max_width, Some(150.0));
        assert_eq!(right.max_height, Some(240.0));
    }

    #[test]
    fn boundary_padding_reduces_available_max_size() {
        let mut options = base_options(Placement::Bottom);
        options.auto_max_size = true;
        options.boundary = Boundary::Viewport;
        options.boundary_padding = 10.0;

        let result = compute_position(&anchor(), &floating(), &viewport(), &options);
        assert_eq!(result.max_width, Some(280.0));
        assert_eq!(result.max_height, Some(100.0));
    }
}
