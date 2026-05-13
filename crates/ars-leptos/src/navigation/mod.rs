//! Navigation components for the Leptos adapter.
//!
//! Components in this module help users move between sections, pages, or
//! views: tabs, accordions, breadcrumbs, paginators, navigation menus, and
//! related primitives.

/// Tabs adapter — renders the agnostic [`ars_components::navigation::tabs`]
/// machine as a single Leptos `<Tabs>` component owning a tablist, tabs,
/// indicator, panels, optional close triggers, and an optional reorder
/// live region.
pub mod tabs;
