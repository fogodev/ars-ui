//! WAI-ARIA attribute types, role definitions, and shared helpers.
//!
//! - [`role::AriaRole`] — complete WAI-ARIA 1.2 role enum with validation methods
//! - [`attribute::AriaAttribute`] — typed ARIA states and properties with serialization
//! - [`apply`] — role assignment helpers for `connect()` implementations
//! - [`state`] — common ARIA state transition helpers

/// Role assignment helpers for `connect()` implementations.
pub mod apply;
/// Typed WAI-ARIA 1.2 states and properties with serialization support.
pub mod attribute;
/// Complete WAI-ARIA 1.2 role enum with validation methods.
pub mod role;
/// Common ARIA state transition helpers.
pub mod state;
