//! Compile-time contract coverage for crate-root re-exports.

#[test]
fn orientation_reexport_pass_tests() {
    let cases = trybuild::TestCases::new();

    cases.pass("tests/pass/orientation_reexport.rs");
}
