//! Ignored nightly property-based tests for ars-components state machines.

#[path = "proptest_state_machines/input.rs"]
mod input;

#[path = "proptest_state_machines/date_time.rs"]
mod date_time;

#[path = "proptest_state_machines/layout.rs"]
mod layout;

#[path = "proptest_state_machines/overlay.rs"]
mod overlay;

#[path = "proptest_state_machines/utility.rs"]
mod utility;
