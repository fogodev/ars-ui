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
//! - [`Collection`] — read-only, ordered collection trait.
//! - [`CollectionItem`] — trait for items stored in collections.
//! - [`CollectionBuilder`] — fluent builder for constructing collections.
//! - [`StaticCollection`] — in-memory `Collection` implementation.
//! - [`TreeCollection`] — hierarchical `Collection` with expand/collapse.
//! - [`TreeItemConfig`] — configuration for tree item construction.
//! - [`selection`] — selection enums and state for collection-based components.
//! - [`navigation`] — disabled-aware navigation helpers for collection widgets.
//! - [`AsyncLoadingState`] — loading phase for async/paginated collections.
//! - [`AsyncCollection`] — paginated collection that grows as pages are fetched.
//! - [`AsyncLoader`] — trait for fetching pages of data from an async source.
//! - [`LoadResult`] — result of a single page fetch.
//! - [`FilteredCollection`] — predicate-based filtering view.
//! - [`SortedCollection`] — comparator-based sorting view.
//! - [`SortDirection`] — ascending or descending sort order.
//! - [`SortDescriptor`] — column + direction for table sorting.
//! - [`CollectionError`] — error from an async page load.
//! - [`typeahead`] — type-ahead / type-select state machine for keyboard search.
//! - [`Virtualizer`] — visible-range and scroll math for virtualized rendering.
//! - [`LayoutStrategy`] — sizing strategy used by [`Virtualizer`].
//! - [`ScrollAlign`] — alignment mode for programmatic scrolling.
//! - [`VirtualLayout`] — extension trait for custom vertical layout engines.
//! - [`HorizontalVirtualLayout`] — optional extension trait for horizontal layout engines.
//! - [`normalize_scroll_left_rtl`] — RTL scroll normalization for cross-browser consistency.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::std_instead_of_core)]

extern crate alloc;

/// Async collection with pagination and loading state management.
pub mod async_collection;
/// Async data loading traits and result types.
pub mod async_loader;
/// Fluent builder for constructing [`StaticCollection`] instances.
pub mod builder;
/// Core collection traits: [`Collection`] and [`CollectionItem`].
pub mod collection;
/// Predicate-based filtering view over a [`Collection`].
pub mod filtered_collection;
/// Stable node identifiers for collections.
pub mod key;
/// Disabled-aware navigation helpers for collection widgets.
pub mod navigation;
/// Node types and structural metadata for collection items.
pub mod node;
/// Selection enums and state for collection-based components.
pub mod selection;
/// Comparator-based sorting view over a [`Collection`].
pub mod sorted_collection;
/// In-memory collection backed by `Vec` and `IndexMap`.
pub mod static_collection;
/// Hierarchical collection with expand/collapse for tree-based components.
pub mod tree_collection;
/// Type-ahead / type-select state machine for keyboard search in collections.
pub mod typeahead;
/// Extension trait for custom virtualization layout engines.
pub mod virtual_layout;
/// Virtualized rendering range and scroll math.
pub mod virtualization;

pub use async_collection::{AsyncCollection, AsyncLoadingState};
pub use async_loader::{AsyncLoader, CollectionError, LoadResult};
pub use builder::CollectionBuilder;
pub use collection::{Collection, CollectionItem};
pub use filtered_collection::FilteredCollection;
pub use key::Key;
pub use node::{Node, NodeType};
pub use selection::DisabledBehavior;
pub use sorted_collection::{SortDescriptor, SortDirection, SortedCollection};
pub use static_collection::StaticCollection;
pub use tree_collection::{TreeCollection, TreeItemConfig};
pub use virtual_layout::{HorizontalVirtualLayout, VirtualLayout};
pub use virtualization::{LayoutStrategy, ScrollAlign, Virtualizer, normalize_scroll_left_rtl};
