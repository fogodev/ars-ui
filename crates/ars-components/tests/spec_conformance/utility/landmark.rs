use super::*;

#[test]
fn landmark_anatomy_matches_spec() {
    assert_anatomy("landmark", &[(utility_core::landmark::Part::Root, "root")]);
}
