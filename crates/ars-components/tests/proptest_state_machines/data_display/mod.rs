//! Ignored nightly property-based tests for `crates/ars-components/src/data_display/*`.
//!
//! Each stateful or complex data-display component owns a sibling module
//! holding its arbitraries and `proptest!` block. Stateless components
//! (`Badge`, `Meter`, `Skeleton`, and `Stat`) have no state-machine invariants
//! to property-test; their connect-API contracts are covered by unit,
//! snapshot, and spec-conformance tests.

mod avatar;
mod grid_list;
mod marquee;
mod progress;
mod rating_group;
mod table;
mod tag_group;
