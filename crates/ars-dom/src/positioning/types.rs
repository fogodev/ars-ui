//! Positioning engine data types.
//!
//! Defines the enums, structs, and helper methods consumed by the positioning
//! algorithm to compute where floating elements (popovers, tooltips, menus)
//! should appear relative to their anchor elements.

#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
use std::{any::Any, fmt};

use ars_i18n::ResolvedDirection;

// ---------------------------------------------------------------------------
// Platform-conditional element reference
// ---------------------------------------------------------------------------

/// Platform-conditional type-erased smart pointer.
/// On `wasm32` targets, uses `Rc` (single-threaded browser environment).
/// On native targets, uses `Arc` for thread safety.
#[cfg(target_arch = "wasm32")]
pub(crate) type ElementRef = Rc<dyn Any>;

/// Platform-conditional type-erased smart pointer.
/// On `wasm32` targets, uses `Rc` (single-threaded browser environment).
/// On native targets, uses `Arc` for thread safety.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) type ElementRef = Arc<dyn Any + Send + Sync>;

// ---------------------------------------------------------------------------
// Core enums
// ---------------------------------------------------------------------------

/// The four sides of a rectangle, used for placement direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// Top edge.
    Top,
    /// Right edge.
    Right,
    /// Bottom edge.
    Bottom,
    /// Left edge.
    Left,
}

/// Cross-axis alignment within a placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
    /// Align to the start (top or left, depending on axis).
    Start,
    /// Center alignment (default for unaligned placements).
    Center,
    /// Align to the end (bottom or right, depending on axis).
    End,
}

/// The axis along which a placement is oriented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Left-right axis (used by Left/Right placements).
    Horizontal,
    /// Top-bottom axis (used by Top/Bottom placements).
    Vertical,
}

/// Placement options for floating elements relative to their anchor.
/// The first word is the side of the anchor the floating element appears on.
/// The optional second word is the alignment along that side.
///
/// **Auto variants**: `Auto`, `AutoStart`, `AutoEnd` let the positioning engine
/// choose the side with the most available space. The optional alignment
/// (`Start`/`End`) is preserved after the side is resolved.
///
/// **Logical variants**: `Start`, `End`, `StartTop`, `StartBottom`, `EndTop`,
/// `EndBottom` resolve based on `ctx.dir` (LTR: Start->Left, RTL: Start->Right).
/// This enables correct popover/tooltip placement in bidirectional layouts without
/// requiring the consumer to check text direction manually.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    /// Bottom side, centered (default).
    #[default]
    Bottom,
    /// Bottom side, start-aligned.
    BottomStart,
    /// Bottom side, end-aligned.
    BottomEnd,
    /// Top side, centered.
    Top,
    /// Top side, start-aligned.
    TopStart,
    /// Top side, end-aligned.
    TopEnd,
    /// Left side, centered.
    Left,
    /// Left side, start-aligned.
    LeftStart,
    /// Left side, end-aligned.
    LeftEnd,
    /// Right side, centered.
    Right,
    /// Right side, start-aligned.
    RightStart,
    /// Right side, end-aligned.
    RightEnd,
    /// Positioning engine chooses the side with the most available space.
    Auto,
    /// Auto side selection, aligned to the start of that side.
    AutoStart,
    /// Auto side selection, aligned to the end of that side.
    AutoEnd,
    /// Logical inline-start side (LTR: Left, RTL: Right).
    Start,
    /// Logical inline-end side (LTR: Right, RTL: Left).
    End,
    /// Logical inline-start side, aligned to the top.
    StartTop,
    /// Logical inline-start side, aligned to the bottom.
    StartBottom,
    /// Logical inline-end side, aligned to the top.
    EndTop,
    /// Logical inline-end side, aligned to the bottom.
    EndBottom,
}

impl Placement {
    /// Returns the opposite placement (used by the flip algorithm).
    /// Handles both resolved and logical variants. Logical Start/End are swapped.
    /// Auto returns Auto (identity).
    #[must_use]
    pub const fn opposite(&self) -> Placement {
        match self {
            Placement::Bottom => Placement::Top,
            Placement::BottomStart => Placement::TopStart,
            Placement::BottomEnd => Placement::TopEnd,
            Placement::Top => Placement::Bottom,
            Placement::TopStart => Placement::BottomStart,
            Placement::TopEnd => Placement::BottomEnd,
            Placement::Left => Placement::Right,
            Placement::LeftStart => Placement::RightStart,
            Placement::LeftEnd => Placement::RightEnd,
            Placement::Right => Placement::Left,
            Placement::RightStart => Placement::LeftStart,
            Placement::RightEnd => Placement::LeftEnd,
            Placement::Start => Placement::End,
            Placement::End => Placement::Start,
            Placement::StartTop => Placement::EndTop,
            Placement::StartBottom => Placement::EndBottom,
            Placement::EndTop => Placement::StartTop,
            Placement::EndBottom => Placement::StartBottom,
            // Auto variants have no meaningful opposite — return as-is.
            Self::Auto | Self::AutoStart | Self::AutoEnd => *self,
        }
    }

    /// Returns the main axis for this placement (Horizontal or Vertical).
    /// Auto variants default to `Axis::Vertical`; the engine resolves the actual axis.
    #[must_use]
    pub const fn main_axis(&self) -> Axis {
        match self {
            // Physical horizontal + logical inline-start/end variants.
            Placement::Left
            | Placement::LeftStart
            | Placement::LeftEnd
            | Placement::Right
            | Placement::RightStart
            | Placement::RightEnd
            | Placement::Start
            | Placement::End
            | Placement::StartTop
            | Placement::StartBottom
            | Placement::EndTop
            | Placement::EndBottom => Axis::Horizontal,
            // Physical vertical + auto variants (auto defaults to vertical).
            Placement::Top
            | Placement::TopStart
            | Placement::TopEnd
            | Placement::Bottom
            | Placement::BottomStart
            | Placement::BottomEnd
            | Placement::Auto
            | Placement::AutoStart
            | Placement::AutoEnd => Axis::Vertical,
        }
    }

    /// Resolves logical (Start/End) and Auto variants into physical placements.
    /// Logical variants resolve based on `dir` (LTR: Start->Left, RTL: Start->Right).
    /// Auto variants are resolved by the positioning engine based on available space
    /// and are not handled here — they pass through unchanged.
    ///
    #[must_use]
    pub fn resolve_logical(&self, dir: ResolvedDirection) -> Placement {
        match (self, dir) {
            (Placement::Start, ResolvedDirection::Ltr)
            | (Placement::End, ResolvedDirection::Rtl) => Placement::Left,
            (Placement::Start, ResolvedDirection::Rtl)
            | (Placement::End, ResolvedDirection::Ltr) => Placement::Right,
            (Placement::StartTop, ResolvedDirection::Ltr)
            | (Placement::EndTop, ResolvedDirection::Rtl) => Placement::LeftStart,
            (Placement::StartTop, ResolvedDirection::Rtl)
            | (Placement::EndTop, ResolvedDirection::Ltr) => Placement::RightStart,
            (Placement::StartBottom, ResolvedDirection::Ltr)
            | (Placement::EndBottom, ResolvedDirection::Rtl) => Placement::LeftEnd,
            (Placement::StartBottom, ResolvedDirection::Rtl)
            | (Placement::EndBottom, ResolvedDirection::Ltr) => Placement::RightEnd,
            (other, _) => *other,
        }
    }

    /// Returns the primary side for this placement.
    /// Logical (Start/End) and Auto variants should be resolved first via
    /// [`resolve_logical()`](Self::resolve_logical) — calling `side()` on them
    /// returns `Side::Bottom` as fallback.
    #[must_use]
    pub const fn side(&self) -> Side {
        match self {
            Placement::Top | Placement::TopStart | Placement::TopEnd => Side::Top,
            Placement::Left | Placement::LeftStart | Placement::LeftEnd => Side::Left,
            Placement::Right | Placement::RightStart | Placement::RightEnd => Side::Right,
            // Bottom variants + Auto/Logical fallback to Side::Bottom.
            _ => Side::Bottom,
        }
    }

    /// The cross-axis alignment (`Start`, `Center`, `End`).
    /// Returns `Alignment::Center` for unaligned placements (Top, Bottom, Left, Right).
    #[must_use]
    pub const fn alignment(&self) -> Alignment {
        match self {
            Self::TopStart
            | Self::BottomStart
            | Self::LeftStart
            | Self::RightStart
            | Self::AutoStart
            | Self::StartTop
            | Self::EndTop => Alignment::Start,
            Self::TopEnd
            | Self::BottomEnd
            | Self::LeftEnd
            | Self::RightEnd
            | Self::AutoEnd
            | Self::StartBottom
            | Self::EndBottom => Alignment::End,
            _ => Alignment::Center,
        }
    }

    /// Extract the side and alignment as a tuple.
    /// Auto/Logical variants return `(Side::Bottom, alignment)` — resolve first.
    #[must_use]
    pub const fn side_and_alignment(&self) -> (Side, Alignment) {
        (self.side(), self.alignment())
    }

    /// Return a new `Placement` with the same alignment but a different side.
    /// Only works on resolved (physical) placements.
    #[must_use]
    pub const fn with_side(&self, new_side: Side) -> Placement {
        let alignment = self.alignment();
        match (new_side, alignment) {
            (Side::Top, Alignment::Start) => Self::TopStart,
            (Side::Top, Alignment::End) => Self::TopEnd,
            (Side::Top, Alignment::Center) => Self::Top,
            (Side::Bottom, Alignment::Start) => Self::BottomStart,
            (Side::Bottom, Alignment::End) => Self::BottomEnd,
            (Side::Bottom, Alignment::Center) => Self::Bottom,
            (Side::Left, Alignment::Start) => Self::LeftStart,
            (Side::Left, Alignment::End) => Self::LeftEnd,
            (Side::Left, Alignment::Center) => Self::Left,
            (Side::Right, Alignment::Start) => Self::RightStart,
            (Side::Right, Alignment::End) => Self::RightEnd,
            (Side::Right, Alignment::Center) => Self::Right,
        }
    }
}

impl fmt::Display for Placement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Top => f.write_str("top"),
            Self::TopStart => f.write_str("top-start"),
            Self::TopEnd => f.write_str("top-end"),
            Self::Bottom => f.write_str("bottom"),
            Self::BottomStart => f.write_str("bottom-start"),
            Self::BottomEnd => f.write_str("bottom-end"),
            Self::Left => f.write_str("left"),
            Self::LeftStart => f.write_str("left-start"),
            Self::LeftEnd => f.write_str("left-end"),
            Self::Right => f.write_str("right"),
            Self::RightStart => f.write_str("right-start"),
            Self::RightEnd => f.write_str("right-end"),
            Self::Auto => f.write_str("auto"),
            Self::AutoStart => f.write_str("auto-start"),
            Self::AutoEnd => f.write_str("auto-end"),
            Self::Start => f.write_str("start"),
            Self::End => f.write_str("end"),
            Self::StartTop => f.write_str("start-top"),
            Self::StartBottom => f.write_str("start-bottom"),
            Self::EndTop => f.write_str("end-top"),
            Self::EndBottom => f.write_str("end-bottom"),
        }
    }
}

// ---------------------------------------------------------------------------
// Positioning strategy
// ---------------------------------------------------------------------------

/// CSS positioning strategy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Strategy {
    /// Position relative to the nearest positioned ancestor.
    #[default]
    Absolute,
    /// Position relative to the viewport (ignores scroll).
    ///
    /// **WARNING: CSS Containment Breakage.** `position: fixed` is positioned relative
    /// to the viewport ONLY when no ancestor creates a containing block. The following
    /// CSS properties on ANY ancestor break fixed positioning, causing the element to
    /// be positioned relative to that ancestor instead:
    /// `transform`, `perspective`, `filter`, `backdrop-filter`,
    /// `will-change: transform|perspective|filter`, `contain: paint|layout|strict`.
    ///
    /// When using `Strategy::Fixed`, ensure the floating element is a direct child of
    /// `<body>` or `ars-portal-root`, and that Portal target ancestors do NOT have
    /// these properties. See Step 0 (Detect Containing Block) and Portal §10 in
    /// `components/layout/portal.md` for runtime detection.
    Fixed,
}

// ---------------------------------------------------------------------------
// Boundary
// ---------------------------------------------------------------------------

/// The boundary against which overflow is detected.
///
/// `ElementRef` is a platform-conditional type-erased smart pointer
/// (`Rc<dyn Any>` on WASM, `Arc<dyn Any>` on native).
/// Wrap in `DomElementRef` for access to the underlying `web_sys::Element`.
#[derive(Debug, Clone, Default)]
pub enum Boundary {
    /// Use the viewport as the overflow boundary (default).
    #[default]
    Viewport,
    /// Use a specific element's bounding rect as the overflow boundary.
    Element(ElementRef),
}

impl PartialEq for Boundary {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Viewport, Self::Viewport) => true,
            #[cfg(target_arch = "wasm32")]
            (Self::Element(a), Self::Element(b)) => Rc::ptr_eq(a, b),
            #[cfg(not(target_arch = "wasm32"))]
            (Self::Element(a), Self::Element(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Geometry structs
// ---------------------------------------------------------------------------

/// Offset applied after initial position computation.
/// `main_axis` pushes along the placement direction (e.g., further from anchor).
/// `cross_axis` shifts perpendicular to the placement direction.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Offset {
    /// Distance along the main axis (placement direction).
    pub main_axis: f64,
    /// Distance along the cross axis (perpendicular to placement).
    pub cross_axis: f64,
}

/// Overflow distances on each side relative to the clipping boundary.
/// Positive values indicate overflow beyond the boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Overflow {
    /// Overflow beyond the top edge.
    pub top: f64,
    /// Overflow beyond the right edge.
    pub right: f64,
    /// Overflow beyond the bottom edge.
    pub bottom: f64,
    /// Overflow beyond the left edge.
    pub left: f64,
}

/// A simple rectangle with position and dimensions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// X coordinate of the top-left corner.
    pub x: f64,
    /// Y coordinate of the top-left corner.
    pub y: f64,
    /// Width of the rectangle.
    pub width: f64,
    /// Height of the rectangle.
    pub height: f64,
}

impl Rect {
    /// Returns the x coordinate of the right edge (`x + width`).
    #[must_use]
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// Returns the y coordinate of the bottom edge (`y + height`).
    #[must_use]
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// Returns the x coordinate of the horizontal center (`x + width / 2`).
    #[must_use]
    pub fn center_x(&self) -> f64 {
        self.x + self.width / 2.0
    }

    /// Returns the y coordinate of the vertical center (`y + height / 2`).
    #[must_use]
    pub fn center_y(&self) -> f64 {
        self.y + self.height / 2.0
    }
}

// ---------------------------------------------------------------------------
// Positioning options and result
// ---------------------------------------------------------------------------

/// Full set of options controlling how a floating element is positioned.
///
/// Note: `PartialEq` is derived even though this struct contains `f64` fields.
/// Rust's `f64` implements `PartialEq` (but not `Eq`) — `NaN != NaN`.
/// For Dioxus memoization and signal equality checks, this is acceptable because
/// positioning options are always finite values in practice.
#[derive(Debug, Clone, PartialEq)]
pub struct PositioningOptions {
    /// Desired placement relative to the anchor.
    pub placement: Placement,

    /// Distance offset from the anchor edge.
    pub offset: Offset,

    /// When true, automatically flip to the opposite side if the floating element
    /// overflows the boundary on the main axis.
    pub flip: bool,

    /// When true, shift the floating element along the cross axis to keep it
    /// within the boundary while maintaining the anchor connection.
    pub shift: bool,

    /// Minimum distance from viewport edge when shifting (cross-axis padding).
    pub shift_padding: f64,

    /// When true, allow the floating element to overlap the anchor.
    pub overlap: bool,

    /// The container that defines the overflow boundary.
    pub boundary: Boundary,

    /// Padding from the clipping boundary (applied on all sides during overflow
    /// detection).
    pub boundary_padding: f64,

    /// CSS positioning strategy for the floating element.
    pub strategy: Strategy,

    /// Layout direction for resolving logical placements (Start/End) to physical
    /// placements (Left/Right). Default: LTR.
    pub dir: ResolvedDirection,

    /// Min distance from arrow to floating element edge.
    pub arrow_padding: f64,

    /// When true, automatically constrain `max-width`/`max-height` so the floating
    /// element does not exceed the available space within the boundary.
    pub auto_max_size: bool,

    /// Optional list of fallback placements to try (in order) when the primary
    /// placement overflows. When this is non-empty, the flip step tries each
    /// fallback before falling back to the direct opposite.
    pub fallback_placements: Vec<Placement>,

    /// When true, reposition on virtual keyboard open/close via `visualViewport`
    /// events. Enable for floating elements containing or triggered by input
    /// fields. Default: false.
    pub keyboard_aware: bool,

    /// When true, try all 12 placements and select the one with the least overflow.
    /// When enabled, the `flip` option is ignored (auto placement already considers
    /// all placements). The `shift` step still applies after auto placement.
    /// This is the most expensive positioning mode — use sparingly.
    pub auto_placement: bool,
}

impl Default for PositioningOptions {
    fn default() -> Self {
        PositioningOptions {
            placement: Placement::default(),
            offset: Offset::default(),
            flip: true,
            shift: true,
            shift_padding: 5.0,
            overlap: false,
            boundary: Boundary::default(),
            boundary_padding: 8.0,
            strategy: Strategy::default(),
            dir: ResolvedDirection::Ltr,
            arrow_padding: 8.0,
            auto_max_size: true,
            fallback_placements: Vec::new(),
            keyboard_aware: false,
            auto_placement: false,
        }
    }
}

/// The computed result of positioning a floating element.
///
/// After computing the result, the adapter MUST write
/// `data-ars-placement={actual_placement}` onto the floating element
/// (see `00-overview.md` §3 for the canonical attribute name).
/// CSS uses this attribute to orient arrow elements.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositioningResult {
    /// Final x coordinate for the floating element.
    pub x: f64,
    /// Final y coordinate for the floating element.
    pub y: f64,
    /// The actual placement after flip/shift adjustments.
    /// May differ from the requested placement if flipping occurred.
    pub actual_placement: Placement,
    /// Arrow x position along the floating element edge.
    pub arrow_x: Option<f64>,
    /// Arrow y position along the floating element edge.
    pub arrow_y: Option<f64>,
    /// The maximum height of the floating element (set when `auto_max_size` is
    /// enabled).
    pub max_height: Option<f64>,
    /// The maximum width of the floating element (set when `auto_max_size` is
    /// enabled).
    pub max_width: Option<f64>,
}

// ---------------------------------------------------------------------------
// Virtual element
// ---------------------------------------------------------------------------

/// A non-DOM anchor for positioning. The `get_rect` callback returns the
/// current client-space bounding rect of the virtual anchor.
///
/// `compute_position()` accepts `anchor: &Rect`, so it already supports
/// positioning relative to arbitrary rectangles instead of DOM elements.
/// `VirtualElement` wraps a closure that produces such a `Rect` on demand,
/// allowing the anchor geometry to change between calls (e.g., tracking a
/// cursor position for context menus or a text selection range).
///
/// **Auto-update limitation:** `auto_update()` uses `ResizeObserver` and
/// scroll listeners, which require real DOM elements. When using a virtual
/// element, the consumer must call `compute_position()` manually whenever
/// the virtual rect changes.
pub struct VirtualElement {
    /// Returns the current bounding rectangle in client-space coordinates.
    pub get_rect: Box<dyn Fn() -> Rect>,
}

impl fmt::Debug for VirtualElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VirtualElement")
            .field("get_rect", &"Fn() -> Rect")
            .finish()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // All 21 placement variants for exhaustive iteration
    // -----------------------------------------------------------------------

    const ALL_PLACEMENTS: [Placement; 21] = [
        Placement::Bottom,
        Placement::BottomStart,
        Placement::BottomEnd,
        Placement::Top,
        Placement::TopStart,
        Placement::TopEnd,
        Placement::Left,
        Placement::LeftStart,
        Placement::LeftEnd,
        Placement::Right,
        Placement::RightStart,
        Placement::RightEnd,
        Placement::Auto,
        Placement::AutoStart,
        Placement::AutoEnd,
        Placement::Start,
        Placement::End,
        Placement::StartTop,
        Placement::StartBottom,
        Placement::EndTop,
        Placement::EndBottom,
    ];

    // -----------------------------------------------------------------------
    // opposite()
    // -----------------------------------------------------------------------

    #[test]
    fn opposite_of_bottom_variants_returns_top_variants() {
        assert_eq!(Placement::Bottom.opposite(), Placement::Top);
        assert_eq!(Placement::BottomStart.opposite(), Placement::TopStart);
        assert_eq!(Placement::BottomEnd.opposite(), Placement::TopEnd);
    }

    #[test]
    fn opposite_of_top_variants_returns_bottom_variants() {
        assert_eq!(Placement::Top.opposite(), Placement::Bottom);
        assert_eq!(Placement::TopStart.opposite(), Placement::BottomStart);
        assert_eq!(Placement::TopEnd.opposite(), Placement::BottomEnd);
    }

    #[test]
    fn opposite_of_left_variants_returns_right_variants() {
        assert_eq!(Placement::Left.opposite(), Placement::Right);
        assert_eq!(Placement::LeftStart.opposite(), Placement::RightStart);
        assert_eq!(Placement::LeftEnd.opposite(), Placement::RightEnd);
    }

    #[test]
    fn opposite_of_right_variants_returns_left_variants() {
        assert_eq!(Placement::Right.opposite(), Placement::Left);
        assert_eq!(Placement::RightStart.opposite(), Placement::LeftStart);
        assert_eq!(Placement::RightEnd.opposite(), Placement::LeftEnd);
    }

    #[test]
    fn opposite_of_logical_start_returns_end() {
        assert_eq!(Placement::Start.opposite(), Placement::End);
        assert_eq!(Placement::StartTop.opposite(), Placement::EndTop);
        assert_eq!(Placement::StartBottom.opposite(), Placement::EndBottom);
    }

    #[test]
    fn opposite_of_logical_end_returns_start() {
        assert_eq!(Placement::End.opposite(), Placement::Start);
        assert_eq!(Placement::EndTop.opposite(), Placement::StartTop);
        assert_eq!(Placement::EndBottom.opposite(), Placement::StartBottom);
    }

    #[test]
    fn opposite_of_auto_variants_returns_identity() {
        assert_eq!(Placement::Auto.opposite(), Placement::Auto);
        assert_eq!(Placement::AutoStart.opposite(), Placement::AutoStart);
        assert_eq!(Placement::AutoEnd.opposite(), Placement::AutoEnd);
    }

    #[test]
    fn opposite_is_involutory_for_all_variants() {
        for p in ALL_PLACEMENTS {
            assert_eq!(
                p.opposite().opposite(),
                p,
                "opposite(opposite({p:?})) should return the original"
            );
        }
    }

    // -----------------------------------------------------------------------
    // main_axis()
    // -----------------------------------------------------------------------

    #[test]
    fn main_axis_vertical_for_top_and_bottom_variants() {
        let vertical = [
            Placement::Top,
            Placement::TopStart,
            Placement::TopEnd,
            Placement::Bottom,
            Placement::BottomStart,
            Placement::BottomEnd,
        ];
        for p in vertical {
            assert_eq!(p.main_axis(), Axis::Vertical, "{p:?} should be Vertical");
        }
    }

    #[test]
    fn main_axis_horizontal_for_left_and_right_variants() {
        let horizontal = [
            Placement::Left,
            Placement::LeftStart,
            Placement::LeftEnd,
            Placement::Right,
            Placement::RightStart,
            Placement::RightEnd,
        ];
        for p in horizontal {
            assert_eq!(
                p.main_axis(),
                Axis::Horizontal,
                "{p:?} should be Horizontal"
            );
        }
    }

    #[test]
    fn main_axis_horizontal_for_logical_variants() {
        let logical = [
            Placement::Start,
            Placement::End,
            Placement::StartTop,
            Placement::StartBottom,
            Placement::EndTop,
            Placement::EndBottom,
        ];
        for p in logical {
            assert_eq!(
                p.main_axis(),
                Axis::Horizontal,
                "{p:?} (logical) should be Horizontal"
            );
        }
    }

    #[test]
    fn main_axis_vertical_for_auto_variants() {
        let auto = [Placement::Auto, Placement::AutoStart, Placement::AutoEnd];
        for p in auto {
            assert_eq!(
                p.main_axis(),
                Axis::Vertical,
                "{p:?} (auto) should default to Vertical"
            );
        }
    }

    // -----------------------------------------------------------------------
    // resolve_logical()
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_logical_ltr_maps_start_to_left_and_end_to_right() {
        assert_eq!(
            Placement::Start.resolve_logical(ResolvedDirection::Ltr),
            Placement::Left
        );
        assert_eq!(
            Placement::End.resolve_logical(ResolvedDirection::Ltr),
            Placement::Right
        );
        assert_eq!(
            Placement::StartTop.resolve_logical(ResolvedDirection::Ltr),
            Placement::LeftStart
        );
        assert_eq!(
            Placement::StartBottom.resolve_logical(ResolvedDirection::Ltr),
            Placement::LeftEnd
        );
        assert_eq!(
            Placement::EndTop.resolve_logical(ResolvedDirection::Ltr),
            Placement::RightStart
        );
        assert_eq!(
            Placement::EndBottom.resolve_logical(ResolvedDirection::Ltr),
            Placement::RightEnd
        );
    }

    #[test]
    fn resolve_logical_rtl_maps_start_to_right_and_end_to_left() {
        assert_eq!(
            Placement::Start.resolve_logical(ResolvedDirection::Rtl),
            Placement::Right
        );
        assert_eq!(
            Placement::End.resolve_logical(ResolvedDirection::Rtl),
            Placement::Left
        );
        assert_eq!(
            Placement::StartTop.resolve_logical(ResolvedDirection::Rtl),
            Placement::RightStart
        );
        assert_eq!(
            Placement::StartBottom.resolve_logical(ResolvedDirection::Rtl),
            Placement::RightEnd
        );
        assert_eq!(
            Placement::EndTop.resolve_logical(ResolvedDirection::Rtl),
            Placement::LeftStart
        );
        assert_eq!(
            Placement::EndBottom.resolve_logical(ResolvedDirection::Rtl),
            Placement::LeftEnd
        );
    }

    #[test]
    fn resolve_logical_passes_through_physical_placements() {
        let physical = [
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
        for p in physical {
            assert_eq!(
                p.resolve_logical(ResolvedDirection::Ltr),
                p,
                "{p:?} should pass through in LTR"
            );
            assert_eq!(
                p.resolve_logical(ResolvedDirection::Rtl),
                p,
                "{p:?} should pass through in RTL"
            );
        }
    }

    #[test]
    fn resolve_logical_passes_through_auto_variants() {
        let auto = [Placement::Auto, Placement::AutoStart, Placement::AutoEnd];
        for p in auto {
            assert_eq!(
                p.resolve_logical(ResolvedDirection::Ltr),
                p,
                "{p:?} should pass through in LTR"
            );
            assert_eq!(
                p.resolve_logical(ResolvedDirection::Rtl),
                p,
                "{p:?} should pass through in RTL"
            );
        }
    }

    // -----------------------------------------------------------------------
    // side()
    // -----------------------------------------------------------------------

    #[test]
    fn side_extracts_correct_side_for_physical_placements() {
        assert_eq!(Placement::Top.side(), Side::Top);
        assert_eq!(Placement::TopStart.side(), Side::Top);
        assert_eq!(Placement::TopEnd.side(), Side::Top);
        assert_eq!(Placement::Bottom.side(), Side::Bottom);
        assert_eq!(Placement::BottomStart.side(), Side::Bottom);
        assert_eq!(Placement::BottomEnd.side(), Side::Bottom);
        assert_eq!(Placement::Left.side(), Side::Left);
        assert_eq!(Placement::LeftStart.side(), Side::Left);
        assert_eq!(Placement::LeftEnd.side(), Side::Left);
        assert_eq!(Placement::Right.side(), Side::Right);
        assert_eq!(Placement::RightStart.side(), Side::Right);
        assert_eq!(Placement::RightEnd.side(), Side::Right);
    }

    #[test]
    fn side_returns_bottom_for_unresolved_variants() {
        let unresolved = [
            Placement::Auto,
            Placement::AutoStart,
            Placement::AutoEnd,
            Placement::Start,
            Placement::End,
            Placement::StartTop,
            Placement::StartBottom,
            Placement::EndTop,
            Placement::EndBottom,
        ];
        for p in unresolved {
            assert_eq!(
                p.side(),
                Side::Bottom,
                "{p:?} (unresolved) should fallback to Side::Bottom"
            );
        }
    }

    // -----------------------------------------------------------------------
    // alignment()
    // -----------------------------------------------------------------------

    #[test]
    fn alignment_returns_center_for_unaligned_placements() {
        let center = [
            Placement::Top,
            Placement::Bottom,
            Placement::Left,
            Placement::Right,
            Placement::Auto,
            Placement::Start,
            Placement::End,
        ];
        for p in center {
            assert_eq!(
                p.alignment(),
                Alignment::Center,
                "{p:?} should have Center alignment"
            );
        }
    }

    #[test]
    fn alignment_returns_start_for_start_aligned_placements() {
        let start = [
            Placement::TopStart,
            Placement::BottomStart,
            Placement::LeftStart,
            Placement::RightStart,
            Placement::AutoStart,
            Placement::StartTop,
            Placement::EndTop,
        ];
        for p in start {
            assert_eq!(
                p.alignment(),
                Alignment::Start,
                "{p:?} should have Start alignment"
            );
        }
    }

    #[test]
    fn alignment_returns_end_for_end_aligned_placements() {
        let end = [
            Placement::TopEnd,
            Placement::BottomEnd,
            Placement::LeftEnd,
            Placement::RightEnd,
            Placement::AutoEnd,
            Placement::StartBottom,
            Placement::EndBottom,
        ];
        for p in end {
            assert_eq!(
                p.alignment(),
                Alignment::End,
                "{p:?} should have End alignment"
            );
        }
    }

    // -----------------------------------------------------------------------
    // side_and_alignment()
    // -----------------------------------------------------------------------

    #[test]
    fn side_and_alignment_returns_consistent_tuple() {
        for p in ALL_PLACEMENTS {
            assert_eq!(
                p.side_and_alignment(),
                (p.side(), p.alignment()),
                "{p:?}: side_and_alignment() should equal (side(), alignment())"
            );
        }
    }

    // -----------------------------------------------------------------------
    // with_side()
    // -----------------------------------------------------------------------

    #[test]
    fn with_side_preserves_alignment() {
        assert_eq!(
            Placement::BottomStart.with_side(Side::Top),
            Placement::TopStart
        );
        assert_eq!(Placement::TopEnd.with_side(Side::Left), Placement::LeftEnd);
        assert_eq!(Placement::Right.with_side(Side::Bottom), Placement::Bottom);
        assert_eq!(
            Placement::LeftStart.with_side(Side::Right),
            Placement::RightStart
        );
    }

    #[test]
    fn with_side_produces_all_twelve_physical_placements() {
        let sides = [Side::Top, Side::Bottom, Side::Left, Side::Right];
        let alignments = [Alignment::Start, Alignment::Center, Alignment::End];

        for side in sides {
            for alignment in alignments {
                // Build a source placement with the desired alignment
                let source = match alignment {
                    Alignment::Start => Placement::BottomStart,
                    Alignment::Center => Placement::Bottom,
                    Alignment::End => Placement::BottomEnd,
                };
                let result = source.with_side(side);
                assert_eq!(result.side(), side, "with_side({side:?}) should set side");
                assert_eq!(
                    result.alignment(),
                    alignment,
                    "with_side should preserve alignment {alignment:?}"
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Display
    // -----------------------------------------------------------------------

    #[test]
    fn display_produces_correct_kebab_case_strings() {
        assert_eq!(Placement::Bottom.to_string(), "bottom");
        assert_eq!(Placement::BottomStart.to_string(), "bottom-start");
        assert_eq!(Placement::BottomEnd.to_string(), "bottom-end");
        assert_eq!(Placement::Top.to_string(), "top");
        assert_eq!(Placement::TopStart.to_string(), "top-start");
        assert_eq!(Placement::TopEnd.to_string(), "top-end");
        assert_eq!(Placement::Left.to_string(), "left");
        assert_eq!(Placement::LeftStart.to_string(), "left-start");
        assert_eq!(Placement::LeftEnd.to_string(), "left-end");
        assert_eq!(Placement::Right.to_string(), "right");
        assert_eq!(Placement::RightStart.to_string(), "right-start");
        assert_eq!(Placement::RightEnd.to_string(), "right-end");
        assert_eq!(Placement::Auto.to_string(), "auto");
        assert_eq!(Placement::AutoStart.to_string(), "auto-start");
        assert_eq!(Placement::AutoEnd.to_string(), "auto-end");
        assert_eq!(Placement::Start.to_string(), "start");
        assert_eq!(Placement::End.to_string(), "end");
        assert_eq!(Placement::StartTop.to_string(), "start-top");
        assert_eq!(Placement::StartBottom.to_string(), "start-bottom");
        assert_eq!(Placement::EndTop.to_string(), "end-top");
        assert_eq!(Placement::EndBottom.to_string(), "end-bottom");
    }

    #[test]
    fn display_covers_all_twenty_one_variants() {
        // Ensure every variant produces a non-empty string.
        for p in ALL_PLACEMENTS {
            let s = p.to_string();
            assert!(
                !s.is_empty(),
                "{p:?} should produce a non-empty Display string"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Default impls
    // -----------------------------------------------------------------------

    #[test]
    fn placement_default_is_bottom() {
        assert_eq!(Placement::default(), Placement::Bottom);
    }

    #[test]
    fn strategy_default_is_absolute() {
        assert_eq!(Strategy::default(), Strategy::Absolute);
    }

    #[test]
    fn boundary_default_is_viewport() {
        assert_eq!(Boundary::default(), Boundary::Viewport);
    }

    #[test]
    fn offset_default_is_zero() {
        let o = Offset::default();
        assert_eq!(o.main_axis, 0.0);
        assert_eq!(o.cross_axis, 0.0);
    }

    #[test]
    fn positioning_options_default_matches_spec() {
        let opts = PositioningOptions::default();
        assert_eq!(opts.placement, Placement::Bottom);
        assert_eq!(opts.offset.main_axis, 0.0);
        assert_eq!(opts.offset.cross_axis, 0.0);
        assert!(opts.flip);
        assert!(opts.shift);
        assert_eq!(opts.shift_padding, 5.0);
        assert!(!opts.overlap);
        assert_eq!(opts.boundary, Boundary::Viewport);
        assert_eq!(opts.boundary_padding, 8.0);
        assert_eq!(opts.strategy, Strategy::Absolute);
        assert_eq!(opts.dir, ResolvedDirection::Ltr);
        assert_eq!(opts.arrow_padding, 8.0);
        assert!(opts.auto_max_size);
        assert!(opts.fallback_placements.is_empty());
        assert!(!opts.keyboard_aware);
        assert!(!opts.auto_placement);
    }

    // -----------------------------------------------------------------------
    // Rect
    // -----------------------------------------------------------------------

    #[test]
    fn rect_computed_edges() {
        let r = Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        assert_eq!(r.right(), 110.0);
        assert_eq!(r.bottom(), 70.0);
        assert_eq!(r.center_x(), 60.0);
        assert_eq!(r.center_y(), 45.0);
    }

    // -----------------------------------------------------------------------
    // Boundary equality
    // -----------------------------------------------------------------------

    #[test]
    fn boundary_viewport_equality() {
        assert_eq!(Boundary::Viewport, Boundary::Viewport);
    }

    #[test]
    fn boundary_viewport_not_equal_to_element() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let elem = Arc::new(42_i32) as Arc<dyn Any + Send + Sync>;
            assert_ne!(Boundary::Viewport, Boundary::Element(elem));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let elem = Rc::new(42_i32) as Rc<dyn Any>;
            assert_ne!(Boundary::Viewport, Boundary::Element(elem));
        }
    }

    #[test]
    fn boundary_element_equality_is_pointer_based() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let a = Arc::new(42_i32) as Arc<dyn Any + Send + Sync>;
            let b = Arc::clone(&a);
            let c = Arc::new(42_i32) as Arc<dyn Any + Send + Sync>;
            // Same pointer
            assert_eq!(Boundary::Element(Arc::clone(&a)), Boundary::Element(b));
            // Different pointer, same value — NOT equal
            assert_ne!(Boundary::Element(a), Boundary::Element(c));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let a = Rc::new(42_i32) as Rc<dyn Any>;
            let b = Rc::clone(&a);
            let c = Rc::new(42_i32) as Rc<dyn Any>;
            assert_eq!(Boundary::Element(Rc::clone(&a)), Boundary::Element(b));
            assert_ne!(Boundary::Element(a), Boundary::Element(c));
        }
    }

    // -----------------------------------------------------------------------
    // VirtualElement
    // -----------------------------------------------------------------------

    #[test]
    fn virtual_element_produces_rect_through_callback() {
        let ve = VirtualElement {
            get_rect: Box::new(|| Rect {
                x: 10.0,
                y: 20.0,
                width: 100.0,
                height: 50.0,
            }),
        };
        let rect = (ve.get_rect)();
        assert_eq!(
            rect,
            Rect {
                x: 10.0,
                y: 20.0,
                width: 100.0,
                height: 50.0,
            }
        );
    }

    #[test]
    fn virtual_element_repeated_calls_may_return_different_rects() {
        use std::{cell::Cell, rc::Rc};

        let cursor_x = Rc::new(Cell::new(0.0));
        let cursor_y = Rc::new(Cell::new(0.0));

        let ve = VirtualElement {
            get_rect: Box::new({
                let cx = Rc::clone(&cursor_x);
                let cy = Rc::clone(&cursor_y);
                move || Rect {
                    x: cx.get(),
                    y: cy.get(),
                    width: 0.0,
                    height: 0.0,
                }
            }),
        };

        let first = (ve.get_rect)();
        assert_eq!(first.x, 0.0);
        assert_eq!(first.y, 0.0);

        cursor_x.set(150.0);
        cursor_y.set(300.0);

        let second = (ve.get_rect)();
        assert_eq!(second.x, 150.0);
        assert_eq!(second.y, 300.0);
        assert_ne!(first, second);
    }

    #[test]
    fn virtual_element_debug_does_not_panic() {
        let ve = VirtualElement {
            get_rect: Box::new(|| Rect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            }),
        };
        let debug_str = format!("{ve:?}");
        assert!(debug_str.contains("VirtualElement"));
        assert!(debug_str.contains("Fn() -> Rect"));
    }
}
