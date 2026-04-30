//! Integration test: drive `cargo xtask spec compile-snippets` against the
//! real spec corpus and assert a clean baseline.
//!
//! The unit tests inside `xtask::spec::compile_snippets::tests` cover the
//! parser, walker, and rewriter in isolation against synthetic fixtures.
//! This test closes the loop by driving `execute()` against the actual
//! `spec/` directory shipped in the repo.
//!
//! Without this test a future spec edit that introduces a Rust syntax bug
//! in a `rust`-tagged code block would only be caught when CI runs the
//! `spec-compile-snippets` step. The integration test catches the same
//! regression locally during `cargo test -p xtask`, with a
//! deterministic-and-actionable failure message tied to the offending
//! `<file:line>` location.

use std::path::PathBuf;

use xtask::{manifest::SpecRoot, spec::compile_snippets};

/// Locate the workspace `spec/` directory by walking up from `CARGO_MANIFEST_DIR`.
///
/// `xtask` lives at `<workspace>/xtask`, so the spec root is one directory
/// above the crate manifest. Resolving via `CARGO_MANIFEST_DIR` keeps the
/// test usable from any working directory and from cargo-nextest's
/// per-test sandbox.
fn workspace_spec_root() -> SpecRoot {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let workspace = manifest_dir
        .parent()
        .expect("xtask crate has a parent workspace directory");

    SpecRoot::discover(workspace).expect("spec/manifest.toml is reachable from the workspace root")
}

#[test]
fn real_spec_corpus_has_no_compile_snippets_findings() {
    let root = workspace_spec_root();

    let report =
        compile_snippets::execute(&root, false).expect("compile-snippets should not error out");

    assert!(
        report.starts_with("ok:"),
        "real spec corpus introduced compile-snippets findings:\n\n{report}\n\n\
         To inspect locally: `cargo xtask spec compile-snippets`. \
         To auto-tag intentionally-partial blocks with `rust,no_check`: \
         `cargo xtask spec compile-snippets --fix`."
    );
}
