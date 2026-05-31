//! Spec-conformance tests for `crates/ars-components/src/navigation/*`.
//!
//! Each test pulls the expected anatomy from the corresponding component
//! spec's §2 / §3 anatomy tables and asserts the impl's `Part` enum matches
//! the declared `(scope, part-name)` ordering.

mod accordion;
mod breadcrumbs;
mod link;
mod navigation_menu;
mod pagination;
mod steps;
mod tabs;
mod tree_view;
