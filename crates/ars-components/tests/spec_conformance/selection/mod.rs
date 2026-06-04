//! Spec-conformance tests for `crates/ars-components/src/selection/*`.
//!
//! Each selection component module asserts the impl's `Part` enum matches the
//! spec's declared anatomy.

use ars_collections::Key;

use super::helper::assert_anatomy;

mod autocomplete;
mod combobox;
mod context_menu;
mod listbox;
mod menu;
mod menu_bar;
mod segment_group;
mod select;
mod tags_input;
