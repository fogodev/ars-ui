//! Selection component machines.
//!
//! Components in this module present one or more choices backed by
//! collection metadata, selection state, keyboard navigation, and type-ahead
//! matching while leaving rendering and live DOM operations to framework
//! adapters.

/// Combobox component machine.
pub mod combobox;

/// Listbox component machine.
pub mod listbox;

/// Menu component machine.
pub mod menu;

/// Select component machine.
pub mod select;
