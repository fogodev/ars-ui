//! Keyboard shortcut descriptors and platform-normalized modifier matching.

/// Shortcut descriptors and platform-aware modifier matching primitives.
pub mod shortcuts;

pub use shortcuts::{DomEvent, KeyModifiers, KeyboardShortcut, Platform};
