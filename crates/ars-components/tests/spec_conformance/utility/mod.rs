//! Spec-conformance tests for `crates/ars-components/src/utility/*`.
//!
//! Each test pulls the expected anatomy from the corresponding component
//! spec's §2 anatomy table and asserts the impl's `Part` enum matches.

use ars_collections::Key;
use ars_components::utility as utility_core;
use ars_core::{Env, HtmlAttr, Service};

use super::helper::assert_anatomy;

mod action_group;
mod ars_provider;
mod as_child;
mod button;
mod client_only;
mod dismissable;
mod download_trigger;
mod drop_zone;
mod error_boundary;
mod field;
mod fieldset;
mod focus_ring;
mod focus_scope;
mod form;
mod form_submit;
mod group;
mod heading;
mod highlight;
mod keyboard;
mod landmark;
mod live_region;
mod separator;
mod swap;
mod toggle;
mod toggle_button;
mod toggle_group;
mod visually_hidden;
mod z_index_allocator;
