use super::*;

#[test]
fn focus_ring_anatomy_matches_spec() {
    assert_anatomy(
        "focus-ring",
        &[(utility_core::focus_ring::Part::Root, "root")],
    );
}
