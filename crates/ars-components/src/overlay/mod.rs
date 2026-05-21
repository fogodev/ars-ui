//! Overlay component machines.

/// Alert dialog machine.
pub mod alert_dialog;

/// Shared DOM-free overlay positioning configuration.
pub mod positioning;

/// Dialog machine.
pub mod dialog;

/// Floating panel machine.
pub mod floating_panel;

/// Hover card machine.
pub mod hover_card;

/// Popover machine.
pub mod popover;

/// Presence machine.
pub mod presence;

/// Toast notification component (per-toast machine + provider/manager
/// coordinator). See `spec/components/overlay/toast.md`.
pub mod toast;

/// Tooltip machine.
pub mod tooltip;
