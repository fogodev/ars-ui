//! Shared configuration for the property-based state-machine tests.
//!
//! Centralises two concerns that every `proptest!` block in this harness
//! cares about:
//!
//! 1. **Case count.** Reads the `PROPTEST_CASES` environment variable so
//!    the nightly `extended-proptest` job (10,000 cases) and local runs
//!    (1,000 cases by default) share one knob.
//! 2. **Failure persistence.** Redirects the `*.proptest-regressions`
//!    seed files to a single per-crate `proptest-regressions/`
//!    directory at the crate root instead of dropping them next to test
//!    sources. Default proptest behaviour clutters the source tree with
//!    a sibling file per test module; this single-file layout is the
//!    same content but lives next to `Cargo.toml` where bookkeeping
//!    files belong. The file is committed (per proptest convention) so
//!    every developer's runs benefit from previously-shrunk failures.
//!
//! Use as `#![proptest_config(super::common::proptest_config())]` inside
//! every `proptest!` block in this harness.

use proptest::test_runner::{Config, FileFailurePersistence};

/// Builds a `ProptestConfig` with the workspace-standard case count and
/// failure-persistence layout.
///
/// The persistence target is a single file at
/// `<crate-root>/proptest-regressions/state-machines.txt` — proptest
/// internally namespaces seeds by test name so multiple modules can
/// safely share one file.
#[must_use]
pub(super) fn proptest_config() -> Config {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);

    Config {
        cases,
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(
            "proptest-regressions/state-machines.txt",
        ))),
        ..Config::default()
    }
}
