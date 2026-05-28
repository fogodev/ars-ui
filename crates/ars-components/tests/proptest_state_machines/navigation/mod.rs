//! Ignored nightly property-based tests for `crates/ars-components/src/navigation/*`.
//!
//! Mirrors the per-component layout used by `input/`, `overlay/`, and
//! `utility/`: each component owns a sibling module holding its arbitraries
//! and `proptest!` block. Strategies shared by more than one component live
//! here so they are defined once.
//!
//! Each `proptest!` block is `#[ignore]`d so the default `cargo test` run
//! skips them; the nightly `extended-proptest` job clears the ignore filter
//! and runs them with a higher case count via `PROPTEST_CASES`.

use ars_collections::Key;
use ars_core::{Direction, Orientation};
use proptest::prelude::*;

mod accordion;
mod pagination;
mod steps;
mod tabs;
mod tree_view;
// `breadcrumbs` is a stateless component (no `Machine`/`State`/`Event`), so it
// has no state-machine invariants to property-test. Its connect-API contract
// is covered by the unit and spec-conformance suites instead.

/// Small key universe shared by the tabs and accordion strategies so the
/// collision / registration paths get exercised. `TreeView` uses its own
/// fixed-shape key set (see `tree_view.rs`).
fn arb_key() -> impl Strategy<Value = Key> {
    prop_oneof![
        Just(Key::str("a")),
        Just(Key::str("b")),
        Just(Key::str("c")),
        Just(Key::str("d")),
        Just(Key::Int(0)),
        Just(Key::Int(1)),
    ]
}

/// Orientation shared by the tabs, accordion, and steps props strategies.
fn arb_orientation() -> impl Strategy<Value = Orientation> {
    prop_oneof![Just(Orientation::Horizontal), Just(Orientation::Vertical)]
}

/// Writing direction shared by the tabs and accordion strategies.
fn arb_direction() -> impl Strategy<Value = Direction> {
    prop_oneof![
        Just(Direction::Ltr),
        Just(Direction::Rtl),
        Just(Direction::Auto),
    ]
}
