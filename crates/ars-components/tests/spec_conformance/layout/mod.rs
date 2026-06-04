//! Spec-conformance tests for `crates/ars-components/src/layout/*`.
//!
//! Each component module asserts the impl's `Part` enum matches the spec's
//! declared anatomy.

use super::helper::assert_anatomy;

mod aspect_ratio;
mod carousel;
mod center;
mod collapsible;
mod frame;
mod grid;
mod portal;
mod scroll_area;
mod splitter;
mod stack;
mod toolbar;
