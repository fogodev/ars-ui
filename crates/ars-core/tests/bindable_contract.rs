//! Compile-time contract coverage for `Bindable` and `BindableValue`.

#[test]
fn bindable_contract_pass_tests() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/pass/*.rs");
}
