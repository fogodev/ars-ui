use super::*;

#[test]
fn keyboard_anatomy_matches_spec() {
    assert_anatomy("keyboard", &[(utility_core::keyboard::Part::Root, "root")]);
}
