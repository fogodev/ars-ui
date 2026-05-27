//! Ignored nightly property-based tests for `crates/ars-components/src/input/*`.
//!
//! Mirrors the per-component layout used by `overlay/` and `utility/`: each
//! component owns a sibling module holding its arbitraries and `proptest!`
//! block. Strategies shared by more than one component live here so they are
//! defined once.

use proptest::prelude::*;

mod checkbox;
mod checkbox_group;
mod editable;
mod number_input;
mod password_input;
mod pin_input;
mod radio_group;
mod range_slider;
mod search_input;
mod slider;
mod switch;
mod text_field;
mod textarea;
// `file_trigger` is a stateless component (no `Machine`/`State`/`Event`), so it
// has no state-machine invariants to property-test. Its connect-API contract is
// covered by the unit and spec-conformance suites instead.

/// Short lowercase strings used as text-input values across the text-bearing
/// input components (text-field, textarea, editable, password, search).
fn arb_short_text() -> impl Strategy<Value = String> {
    "[a-z]{0,16}".prop_map(String::from)
}

/// Discrete slider steps shared by the single-thumb and range sliders.
fn arb_slider_step() -> impl Strategy<Value = f64> {
    prop_oneof![Just(1.0), Just(2.0), Just(5.0), Just(10.0)]
}
