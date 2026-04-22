//! DOM utilities for focus management, scroll control, z-index allocation,
//! overlay stacking, and platform feature detection.
//!
//! This crate provides browser-level helpers shared across framework adapters,
//! including focus management, scroll lock management for modal overlays,
//! overlay stack registry for nested overlay dismissal, and platform capability
//! detection.

// Many ars-dom functions have cfg-gated web implementations that call
// web_sys/js_sys APIs at runtime. The non-web stubs look const-eligible
// to clippy, but the web paths cannot be const.
#![expect(
    clippy::missing_const_for_fn,
    reason = "cfg-gated web impls prevent const fn"
)]

mod announcer;
mod debug;
pub mod focus;
pub mod media;
pub mod modality;
pub mod overlay_stack;
pub mod portal;
pub mod positioning;
mod scroll;
pub mod scroll_lock;
pub mod z_index;

#[cfg(feature = "web")]
pub use announcer::ensure_dom;
pub use focus::{
    FocusScope, FocusedElement, focus_body, focus_element_by_id, focus_first_tabbable,
};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub use focus::{
    document_contains, focus_element, get_first_focusable, get_focusable_elements,
    get_html_element_by_id, get_last_focusable,
};
pub use media::{
    ColorScheme, is_forced_colors_active, prefers_color_scheme, prefers_high_contrast,
    prefers_reduced_motion, prefers_reduced_transparency,
};
pub use modality::ModalityManager;
pub use overlay_stack::{
    OverlayEntry, contains_overlay, is_above, is_topmost, overlay_count, overlays_above,
    push_overlay, remove_overlay, reset_overlay_stack, topmost_overlay,
};
#[cfg(feature = "web")]
pub use portal::{
    ensure_portal_mount_root, get_or_create_portal_root, remove_inert_from_siblings,
    set_background_inert, supports_inert,
};
pub use positioning::{
    Alignment, Axis, Boundary, Offset, Overflow, Placement, PositioningOptions, PositioningResult,
    Rect, Side, Strategy, VirtualElement, client_point_to_local_space, client_rect_to_local_space,
    compute_position,
};
#[cfg(feature = "web")]
pub use positioning::{
    find_containing_block_ancestor, offset_parent_rect,
    warn_if_floating_element_has_containment_issue, warn_if_portal_target_has_containing_block,
};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub use positioning::{viewport_height, viewport_rect, viewport_width};
pub use scroll::{ScrollIntoViewOptions, ScrollLogicalPosition, supports_scroll_into_view_options};
#[cfg(feature = "web")]
pub use scroll::{nearest_scrollable_ancestor, scroll_into_view_if_needed, scrollable_ancestors};
pub use scroll_lock::{
    ScrollLockManager, acquire, depth, is_locked, prevent_scroll, release, restore_scroll,
};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub use scroll_lock::{needs_ios_workaround, scrollbar_width};
pub use z_index::{ZIndexAllocator, next_z_index, reset_z_index, supports_top_layer};

/// Describes the platform capabilities available to the current runtime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformFeatures {
    /// `true` when running in a web browser environment with DOM access.
    pub web: bool,

    /// `true` when running in server-side rendering mode without DOM access.
    pub ssr: bool,
}
