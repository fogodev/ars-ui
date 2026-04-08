//! Collection and selection abstractions for list-based components.
//!
//! This crate provides the core collection data types and selection state used
//! by components that render collections of items (e.g. select, menu, listbox,
//! tabs, tree-view).
//!
//! # Core types
//!
//! - [`Key`] — stable node identifier (string or integer).
//! - [`NodeType`] — structural role of a node (item, section, header, separator).
//! - [`Node`] — a single node wrapping user data with structural metadata.
//! - [`Selection`] — set of currently selected items.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::std_instead_of_core)]

extern crate alloc;

/// Stable node identifiers for collections.
pub mod key;
/// Node types and structural metadata for collection items.
pub mod node;

use alloc::vec::Vec;

pub use key::Key;
pub use node::{Node, NodeType};

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
