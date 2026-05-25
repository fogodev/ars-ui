use super::*;

#[test]
fn heading_anatomy_matches_spec() {
    assert_anatomy("heading", &[(utility_core::heading::Part::Root, "root")]);
}
