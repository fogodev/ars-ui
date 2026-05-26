//! Selection component machines.
//!
//! Components in this module present one or more choices backed by
//! collection metadata, selection state, keyboard navigation, and type-ahead
//! matching while leaving rendering and live DOM operations to framework
//! adapters.

/// Autocomplete component machine.
pub mod autocomplete;

/// Combobox component machine.
pub mod combobox;

/// Context menu component machine.
pub mod context_menu;

/// Listbox component machine.
pub mod listbox;

/// Menu component machine.
pub mod menu;

/// Menu bar component machine.
pub mod menu_bar;

/// Select component machine.
pub mod select;
