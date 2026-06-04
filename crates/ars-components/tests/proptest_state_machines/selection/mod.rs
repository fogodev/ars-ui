//! Property-based tests for `crates/ars-components/src/selection/*`.
//!
//! Each stateful or complex selection component owns a sibling module holding
//! its arbitraries, invariants, and `proptest!` block.

mod autocomplete;
mod combobox;
mod common;
mod context_menu;
mod listbox;
mod menu;
mod menu_bar;
mod segment_group;
mod select;
mod tags_input;
