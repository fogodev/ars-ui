//! Positioning engine types and algorithms for floating elements.
//!
//! This module provides the data types consumed by the positioning algorithm
//! to compute where floating elements (popovers, tooltips, menus) should be
//! placed relative to their anchor elements.

mod compute;
mod overflow;
mod types;
mod viewport;

pub use compute::compute_position;
pub use types::{
    Alignment, Axis, Boundary, Offset, Overflow, Placement, PositioningOptions, PositioningResult,
    Rect, Side, Strategy, VirtualElement,
};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub use viewport::{viewport_height, viewport_rect, viewport_width};
