//! DOM utilities for focus management, scroll control, and platform feature detection.
//!
//! This crate provides browser-level helpers shared across framework adapters,
//! including focus management, scroll lock management for modal overlays, and
//! platform capability detection.

mod focus;

pub use focus::{
    FocusScope, FocusedElement, focus_body, focus_element_by_id, focus_first_tabbable,
};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub use focus::{
    document_contains, focus_element, get_first_focusable, get_focusable_elements,
    get_html_element_by_id, get_last_focusable,
};

/// An opaque token representing an active scroll lock on the document body.
///
/// Used by modal and overlay components to prevent background scrolling.
/// Scroll locks are reference-counted — the body is only unlocked when all
/// outstanding tokens have been released.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollLockToken;

/// Describes the platform capabilities available to the current runtime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformFeatures {
    /// `true` when running in a web browser environment with DOM access.
    pub web: bool,
    /// `true` when running in server-side rendering mode without DOM access.
    pub ssr: bool,
}
