//! Navigation component machines.
//!
//! Components in this module help the user move between sections, pages, or
//! views: tabbed interfaces, accordions, breadcrumbs, paginators, and so on.

mod key_token;

/// Breadcrumb navigation component.
pub mod breadcrumbs;

/// Link navigation component.
pub mod link;

/// Pagination navigation component.
pub mod pagination;

/// Steps navigation component.
pub mod steps;

/// Accordion navigation component.
pub mod accordion;

/// Tabs component — a tab list paired with associated content panels.
pub mod tabs;

/// Navigation menu component — menubar navigation with delayed submenu content.
pub mod navigation_menu;

/// Tree view component — hierarchical, keyboard-navigable tree (WAI-ARIA tree).
pub mod tree_view;
