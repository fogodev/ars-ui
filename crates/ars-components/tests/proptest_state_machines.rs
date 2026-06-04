//! Ignored nightly property-based tests for ars-components state machines.

// Shared `proptest_config()` helper — every `proptest!` block in the
// modules below uses `super::common::proptest_config()` so the
// `PROPTEST_CASES` env-var handling and centralised failure-persistence
// path stay in one place. See `proptest_state_machines/common.rs` for
// rationale.
#[path = "proptest_state_machines/common.rs"]
mod common;

#[path = "proptest_state_machines/input/mod.rs"]
mod input;

#[path = "proptest_state_machines/date_time/mod.rs"]
mod date_time;

#[path = "proptest_state_machines/data_display/mod.rs"]
mod data_display;

#[path = "proptest_state_machines/layout/mod.rs"]
mod layout;

#[path = "proptest_state_machines/navigation/mod.rs"]
mod navigation;

#[path = "proptest_state_machines/overlay/mod.rs"]
mod overlay;

#[path = "proptest_state_machines/selection.rs"]
mod selection;

#[path = "proptest_state_machines/specialized/mod.rs"]
mod specialized;

#[path = "proptest_state_machines/utility/mod.rs"]
mod utility;
