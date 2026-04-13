//! RTL-aware layout geometry types for logical-to-physical CSS property mapping.
//!
//! Logical properties (`InlineStart`, `InlineEnd`, `BlockStart`, `BlockEnd`) are
//! independent of writing direction.  The [`LogicalSide::to_physical`] and
//! [`LogicalRect::to_physical`] methods resolve them to concrete physical sides
//! given a [`Direction`].

use crate::Direction;

// ────────────────────────────────────────────────────────────────────
// LogicalSide
// ────────────────────────────────────────────────────────────────────

/// A logical side in inline/block flow (independent of writing direction).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LogicalSide {
    /// The start of the inline axis — left in LTR, right in RTL.
    InlineStart,
    /// The end of the inline axis — right in LTR, left in RTL.
    InlineEnd,
    /// The start of the block axis — top in horizontal writing modes.
    BlockStart,
    /// The end of the block axis — bottom in horizontal writing modes.
    BlockEnd,
}

impl LogicalSide {
    /// Convert to a physical side given a writing direction.
    ///
    /// # Panics (debug only)
    ///
    /// Debug-asserts that `dir` is not [`Direction::Auto`]; callers must resolve
    /// `Auto` to `Ltr` or `Rtl` before physical conversion.
    #[must_use]
    pub fn to_physical(self, dir: Direction) -> PhysicalSide {
        debug_assert!(
            dir != Direction::Auto,
            "Direction::Auto must be resolved to Ltr or Rtl before physical conversion"
        );
        match (self, dir) {
            (LogicalSide::InlineStart, Direction::Rtl) => PhysicalSide::Right,
            (LogicalSide::InlineEnd, Direction::Rtl) | (LogicalSide::InlineStart, _) => {
                PhysicalSide::Left
            }
            (LogicalSide::InlineEnd, _) => PhysicalSide::Right,
            (LogicalSide::BlockStart, _) => PhysicalSide::Top,
            (LogicalSide::BlockEnd, _) => PhysicalSide::Bottom,
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// PhysicalSide
// ────────────────────────────────────────────────────────────────────

/// A physical side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhysicalSide {
    /// The left edge.
    Left,
    /// The right edge.
    Right,
    /// The top edge.
    Top,
    /// The bottom edge.
    Bottom,
}

impl PhysicalSide {
    /// Returns the CSS property-value string for this side.
    #[must_use]
    pub const fn as_css(&self) -> &'static str {
        match self {
            PhysicalSide::Left => "left",
            PhysicalSide::Right => "right",
            PhysicalSide::Top => "top",
            PhysicalSide::Bottom => "bottom",
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// LogicalRect
// ────────────────────────────────────────────────────────────────────

/// A logical rectangle with inline-start/end and block-start/end edges.
#[derive(Clone, Copy, Debug, Default)]
pub struct LogicalRect {
    /// Distance from the inline-start edge.
    pub inline_start: f64,
    /// Distance from the inline-end edge.
    pub inline_end: f64,
    /// Distance from the block-start edge.
    pub block_start: f64,
    /// Distance from the block-end edge.
    pub block_end: f64,
}

impl LogicalRect {
    /// Convert to a physical rect given a writing direction.
    ///
    /// In RTL mode the inline-start and inline-end values are swapped so that
    /// `inline_start` maps to `right` and `inline_end` maps to `left`.
    ///
    /// # Panics (debug only)
    ///
    /// Debug-asserts that `dir` is not [`Direction::Auto`]; callers must resolve
    /// `Auto` to `Ltr` or `Rtl` before physical conversion.
    #[must_use]
    pub fn to_physical(&self, dir: Direction) -> PhysicalRect {
        debug_assert!(
            dir != Direction::Auto,
            "Direction::Auto must be resolved to Ltr or Rtl before physical conversion"
        );
        if dir.is_rtl() {
            PhysicalRect {
                left: self.inline_end,
                right: self.inline_start,
                top: self.block_start,
                bottom: self.block_end,
            }
        } else {
            PhysicalRect {
                left: self.inline_start,
                right: self.inline_end,
                top: self.block_start,
                bottom: self.block_end,
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// PhysicalRect
// ────────────────────────────────────────────────────────────────────

/// A physical rectangle with left, right, top, and bottom edges.
#[derive(Clone, Copy, Debug, Default)]
pub struct PhysicalRect {
    /// Distance from the left edge.
    pub left: f64,
    /// Distance from the right edge.
    pub right: f64,
    /// Distance from the top edge.
    pub top: f64,
    /// Distance from the bottom edge.
    pub bottom: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Direction;

    // ── LogicalSide::to_physical ───────────────────────────────────

    #[test]
    fn inline_start_ltr_is_left() {
        assert_eq!(
            LogicalSide::InlineStart.to_physical(Direction::Ltr),
            PhysicalSide::Left,
        );
    }

    #[test]
    fn inline_start_rtl_is_right() {
        assert_eq!(
            LogicalSide::InlineStart.to_physical(Direction::Rtl),
            PhysicalSide::Right,
        );
    }

    #[test]
    fn inline_end_ltr_is_right() {
        assert_eq!(
            LogicalSide::InlineEnd.to_physical(Direction::Ltr),
            PhysicalSide::Right,
        );
    }

    #[test]
    fn inline_end_rtl_is_left() {
        assert_eq!(
            LogicalSide::InlineEnd.to_physical(Direction::Rtl),
            PhysicalSide::Left,
        );
    }

    #[test]
    fn block_start_is_top_regardless_of_direction() {
        assert_eq!(
            LogicalSide::BlockStart.to_physical(Direction::Ltr),
            PhysicalSide::Top,
        );
        assert_eq!(
            LogicalSide::BlockStart.to_physical(Direction::Rtl),
            PhysicalSide::Top,
        );
    }

    #[test]
    fn block_end_is_bottom_regardless_of_direction() {
        assert_eq!(
            LogicalSide::BlockEnd.to_physical(Direction::Ltr),
            PhysicalSide::Bottom,
        );
        assert_eq!(
            LogicalSide::BlockEnd.to_physical(Direction::Rtl),
            PhysicalSide::Bottom,
        );
    }

    // ── PhysicalSide::as_css ───────────────────────────────────────

    #[test]
    fn physical_side_as_css() {
        assert_eq!(PhysicalSide::Left.as_css(), "left");
        assert_eq!(PhysicalSide::Right.as_css(), "right");
        assert_eq!(PhysicalSide::Top.as_css(), "top");
        assert_eq!(PhysicalSide::Bottom.as_css(), "bottom");
    }

    // ── LogicalRect::to_physical ───────────────────────────────────

    #[test]
    fn logical_rect_ltr_maps_directly() {
        let logical = LogicalRect {
            inline_start: 10.0,
            inline_end: 20.0,
            block_start: 5.0,
            block_end: 15.0,
        };
        let physical = logical.to_physical(Direction::Ltr);

        assert_eq!(physical.left, 10.0);
        assert_eq!(physical.right, 20.0);
        assert_eq!(physical.top, 5.0);
        assert_eq!(physical.bottom, 15.0);
    }

    #[test]
    fn logical_rect_rtl_swaps_inline() {
        let logical = LogicalRect {
            inline_start: 10.0,
            inline_end: 20.0,
            block_start: 5.0,
            block_end: 15.0,
        };
        let physical = logical.to_physical(Direction::Rtl);

        // RTL: inline_start -> right, inline_end -> left
        assert_eq!(physical.left, 20.0);
        assert_eq!(physical.right, 10.0);
        // Block sides are unchanged
        assert_eq!(physical.top, 5.0);
        assert_eq!(physical.bottom, 15.0);
    }

    #[test]
    fn logical_rect_block_sides_direction_independent() {
        let logical = LogicalRect {
            inline_start: 0.0,
            inline_end: 0.0,
            block_start: 42.0,
            block_end: 84.0,
        };

        let ltr = logical.to_physical(Direction::Ltr);
        let rtl = logical.to_physical(Direction::Rtl);

        assert_eq!(ltr.top, rtl.top);
        assert_eq!(ltr.bottom, rtl.bottom);
    }

    // ── debug_assert on Direction::Auto ─────────────────────────────

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "Direction::Auto must be resolved")]
    fn logical_side_to_physical_panics_on_auto() {
        let _ = LogicalSide::InlineStart.to_physical(Direction::Auto);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "Direction::Auto must be resolved")]
    fn logical_rect_to_physical_panics_on_auto() {
        let _ = LogicalRect::default().to_physical(Direction::Auto);
    }

    // ── Default impls ──────────────────────────────────────────────

    #[test]
    fn logical_rect_default_is_zero() {
        let r = LogicalRect::default();
        assert_eq!(r.inline_start, 0.0);
        assert_eq!(r.inline_end, 0.0);
        assert_eq!(r.block_start, 0.0);
        assert_eq!(r.block_end, 0.0);
    }

    #[test]
    fn physical_rect_default_is_zero() {
        let r = PhysicalRect::default();
        assert_eq!(r.left, 0.0);
        assert_eq!(r.right, 0.0);
        assert_eq!(r.top, 0.0);
        assert_eq!(r.bottom, 0.0);
    }
}
