//! Ignored nightly property-based tests for `crates/ars-components/src/layout/*`.
//!
//! Stateful layout components own a sibling module holding their arbitraries
//! and `proptest!` block. Stateless layout components (`AspectRatio`,
//! `Center`, `Frame`, `Grid`, and `Stack`) have no state-machine invariants to
//! property-test; their connect-API contracts are covered by unit, snapshot,
//! and spec-conformance tests.

mod carousel;
mod collapsible;
mod portal;
mod scroll_area;
mod splitter;
mod toolbar;
