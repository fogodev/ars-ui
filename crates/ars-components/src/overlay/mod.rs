//! Overlay component machines.

/// Shared DOM-free overlay positioning configuration.
pub mod positioning;

/// Dialog machine.
pub mod dialog;

/// Popover machine.
pub mod popover;

/// Presence machine.
pub mod presence;

/// Toast notification component (per-toast machine + provider/manager
/// coordinator). See `spec/components/overlay/toast.md`.
pub mod toast;

/// Tooltip machine.
pub mod tooltip;
