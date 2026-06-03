//! Spec-conformance tests for `crates/ars-components/src/data_display/*`.
//!
//! Each component module asserts the impl's `Part` enum matches the spec's
//! declared anatomy.

use ars_collections::Key;

use super::helper::assert_anatomy;

mod avatar;
mod badge;
mod grid_list;
mod marquee;
mod meter;
mod progress;
mod rating_group;
mod skeleton;
mod stat;
mod table;
mod tag_group;
