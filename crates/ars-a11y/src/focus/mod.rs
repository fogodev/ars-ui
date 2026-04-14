//! Shared focus-management contracts.

/// Keyboard-modality-based focus-visible tracking.
pub mod ring;
/// Focus-scope options and strategy types.
pub mod scope;
/// Arrow-key navigation across composite widgets.
pub mod zone;

pub use ring::FocusRing;
pub use scope::{FocusScopeBehavior, FocusScopeOptions, FocusStrategy, FocusTarget};
pub use zone::{FocusZone, FocusZoneDirection, FocusZoneOptions};
