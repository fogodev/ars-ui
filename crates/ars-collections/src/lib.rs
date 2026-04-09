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
//! - [`selection`] — selection enums and state for collection-based components.
//! - [`navigation`] — disabled-aware navigation helpers for collection widgets.
//! - [`AsyncLoadingState`] — loading phase for async/paginated collections.
//! - [`AsyncCollection`] — paginated collection that grows as pages are fetched.
//! - [`AsyncLoader`] — trait for fetching pages of data from an async source.
//! - [`LoadResult`] — result of a single page fetch.
//! - [`CollectionError`] — error from an async page load.

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
/// Stable node identifiers for collections.
pub mod key;
/// Disabled-aware navigation helpers for collection widgets.
pub mod navigation;
/// Node types and structural metadata for collection items.
pub mod node;
/// Selection enums and state for collection-based components.
pub mod selection;
/// In-memory collection backed by `Vec` and `IndexMap`.
pub mod static_collection;

pub use async_collection::{AsyncCollection, AsyncLoadingState};
pub use async_loader::{AsyncLoader, CollectionError, LoadResult};
pub use builder::CollectionBuilder;
pub use collection::{Collection, CollectionItem};
pub use key::Key;
pub use node::{Node, NodeType};
pub use selection::DisabledBehavior;
pub use static_collection::StaticCollection;
