//! DOM-free overlay positioning configuration.

use alloc::vec::Vec;
use core::fmt::{self, Display};

/// Placement options for floating overlay elements relative to their anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Placement {
    /// Bottom side, centered.
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

    /// Let the adapter positioning engine choose the side with most available space.
    Auto,

    /// Auto side selection, aligned to the start of that side.
    AutoStart,

    /// Auto side selection, aligned to the end of that side.
    AutoEnd,

    /// Logical inline-start side.
    Start,

    /// Logical inline-end side.
    End,

    /// Logical inline-start side, top-aligned.
    StartTop,

    /// Logical inline-start side, bottom-aligned.
    StartBottom,

    /// Logical inline-end side, top-aligned.
    EndTop,

    /// Logical inline-end side, bottom-aligned.
    EndBottom,
}

impl Placement {
    /// Returns the stable attribute token for this placement.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bottom => "bottom",
            Self::BottomStart => "bottom-start",
            Self::BottomEnd => "bottom-end",
            Self::Top => "top",
            Self::TopStart => "top-start",
            Self::TopEnd => "top-end",
            Self::Left => "left",
            Self::LeftStart => "left-start",
            Self::LeftEnd => "left-end",
            Self::Right => "right",
            Self::RightStart => "right-start",
            Self::RightEnd => "right-end",
            Self::Auto => "auto",
            Self::AutoStart => "auto-start",
            Self::AutoEnd => "auto-end",
            Self::Start => "start",
            Self::End => "end",
            Self::StartTop => "start-top",
            Self::StartBottom => "start-bottom",
            Self::EndTop => "end-top",
            Self::EndBottom => "end-bottom",
        }
    }
}

impl Display for Placement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Offset applied by the adapter positioning engine after initial placement.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Offset {
    /// Distance along the placement direction, away from the anchor edge.
    pub main_axis: f64,

    /// Distance perpendicular to the placement direction.
    pub cross_axis: f64,
}

/// DOM-free options describing how an overlay should be positioned.
#[derive(Clone, Debug, PartialEq)]
pub struct PositioningOptions {
    /// Desired placement relative to the trigger or anchor element.
    pub placement: Placement,

    /// Offset applied after initial placement.
    pub offset: Offset,

    /// Whether the adapter may flip to another side when the preferred side overflows.
    pub flip: bool,

    /// Whether the adapter may shift along the cross axis to stay inside the boundary.
    pub shift: bool,

    /// Minimum distance from the viewport edge when shifting.
    pub shift_padding: f64,

    /// Minimum distance from the arrow to the floating element edge.
    pub arrow_padding: f64,

    /// Whether the adapter should constrain the overlay to available space.
    pub auto_max_size: bool,

    /// Ordered fallback placements the adapter should try before the direct opposite.
    pub fallback_placements: Vec<Placement>,

    /// Whether the adapter should reposition for visual viewport changes.
    pub keyboard_aware: bool,

    /// Whether the adapter should consider all placements and choose the least-overflowing one.
    pub auto_placement: bool,
}

impl Default for PositioningOptions {
    fn default() -> Self {
        Self {
            placement: Placement::default(),
            offset: Offset::default(),
            flip: true,
            shift: true,
            shift_padding: 5.0,
            arrow_padding: 8.0,
            auto_max_size: true,
            fallback_placements: Vec::new(),
            keyboard_aware: false,
            auto_placement: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_as_str_and_display_cover_all_variants() {
        let cases = [
            (Placement::Bottom, "bottom"),
            (Placement::BottomStart, "bottom-start"),
            (Placement::BottomEnd, "bottom-end"),
            (Placement::Top, "top"),
            (Placement::TopStart, "top-start"),
            (Placement::TopEnd, "top-end"),
            (Placement::Left, "left"),
            (Placement::LeftStart, "left-start"),
            (Placement::LeftEnd, "left-end"),
            (Placement::Right, "right"),
            (Placement::RightStart, "right-start"),
            (Placement::RightEnd, "right-end"),
            (Placement::Auto, "auto"),
            (Placement::AutoStart, "auto-start"),
            (Placement::AutoEnd, "auto-end"),
            (Placement::Start, "start"),
            (Placement::End, "end"),
            (Placement::StartTop, "start-top"),
            (Placement::StartBottom, "start-bottom"),
            (Placement::EndTop, "end-top"),
            (Placement::EndBottom, "end-bottom"),
        ];

        for (placement, expected) in cases {
            assert_eq!(placement.as_str(), expected);
            assert_eq!(placement.to_string(), expected);
        }
    }

    #[test]
    fn positioning_options_default_values_are_stable() {
        let options = PositioningOptions::default();

        assert_eq!(options.placement, Placement::Bottom);
        assert_eq!(options.offset, Offset::default());
        assert!(options.flip);
        assert!(options.shift);
        assert_eq!(options.shift_padding, 5.0);
        assert_eq!(options.arrow_padding, 8.0);
        assert!(options.auto_max_size);
        assert!(options.fallback_placements.is_empty());
        assert!(!options.keyboard_aware);
        assert!(!options.auto_placement);
    }
}
