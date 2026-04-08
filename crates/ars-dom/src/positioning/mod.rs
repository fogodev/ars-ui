//! Positioning engine types and algorithms for floating elements.
//!
//! This module provides the data types consumed by the positioning algorithm
//! to compute where floating elements (popovers, tooltips, menus) should be
//! placed relative to their anchor elements.

mod types;

pub use types::{
    Alignment, Axis, Boundary, Offset, Overflow, Placement, PositioningOptions, PositioningResult,
    Rect, Side, Strategy,
};
