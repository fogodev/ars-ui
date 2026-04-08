//! DOM utilities for focus management, scroll control, and platform feature detection.
//!
//! This crate provides browser-level helpers shared across framework adapters,
//! including focus management, scroll lock management for modal overlays, and
//! platform capability detection.

mod focus;
pub mod modality;
pub mod positioning;
mod scroll;
pub mod scroll_lock;

pub use focus::{
    FocusScope, FocusedElement, focus_body, focus_element_by_id, focus_first_tabbable,
};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub use focus::{
    document_contains, focus_element, get_first_focusable, get_focusable_elements,
    get_html_element_by_id, get_last_focusable,
};
pub use modality::ModalityManager;
pub use positioning::{
    Alignment, Axis, Boundary, Offset, Overflow, Placement, PositioningOptions, PositioningResult,
    Rect, Side, Strategy,
};
pub use scroll::{ScrollIntoViewOptions, ScrollLogicalPosition, supports_scroll_into_view_options};
#[cfg(feature = "web")]
pub use scroll::{nearest_scrollable_ancestor, scroll_into_view_if_needed, scrollable_ancestors};
pub use scroll_lock::{
    ScrollLockManager, acquire, depth, is_locked, prevent_scroll, release, restore_scroll,
};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub use scroll_lock::{needs_ios_workaround, scrollbar_width};

/// Describes the platform capabilities available to the current runtime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformFeatures {
    /// `true` when running in a web browser environment with DOM access.
    pub web: bool,
    /// `true` when running in server-side rendering mode without DOM access.
    pub ssr: bool,
}
