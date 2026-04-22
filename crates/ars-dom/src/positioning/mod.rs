//! Positioning engine types and algorithms for floating elements.
//!
//! This module provides the data types consumed by the positioning algorithm
//! to compute where floating elements (popovers, tooltips, menus) should be
//! placed relative to their anchor elements.

mod compute;
mod dom;
mod overflow;
mod types;
mod viewport;

pub use compute::compute_position;
pub use dom::{client_point_to_local_space, client_rect_to_local_space};
#[cfg(feature = "web")]
pub use dom::{
    find_containing_block_ancestor, offset_parent_rect,
    warn_if_floating_element_has_containment_issue, warn_if_portal_target_has_containing_block,
};
pub use types::{
    Alignment, Axis, Boundary, Offset, Overflow, Placement, PositioningOptions, PositioningResult,
    Rect, Side, Strategy, VirtualElement,
};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub use viewport::{viewport_height, viewport_rect, viewport_width};
