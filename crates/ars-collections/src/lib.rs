//! Collection and selection abstractions for list-based components.
//!
//! This crate provides the shared selection state used by components that render
//! collections of items (e.g. select, menu, listbox, tabs, tree-view).

#![no_std]
#![warn(clippy::std_instead_of_core)]

extern crate alloc;

use alloc::vec::Vec;

/// A set of currently selected items in a collection component.
///
/// Tracks which items are selected, supporting single and multiple selection modes.
/// The item type `T` is typically a [`Key`](ars_core) identifying the selected items.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection<T> {
    items: Vec<T>,
}

impl<T> Selection<T> {
    /// Returns a slice of the currently selected items.
    #[must_use]
    pub fn items(&self) -> &[T] {
        &self.items
    }
}
