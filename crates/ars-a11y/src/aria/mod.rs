//! WAI-ARIA attribute types and role definitions.
//!
//! - [`role::AriaRole`] — complete WAI-ARIA 1.2 role enum with validation methods
//! - [`attribute::AriaAttribute`] — typed ARIA states and properties with serialization

/// Typed WAI-ARIA 1.2 states and properties with serialization support.
pub mod attribute;
/// Complete WAI-ARIA 1.2 role enum with validation methods.
pub mod role;
