use super::*;

#[test]
fn live_region_anatomy_matches_spec() {
    assert_anatomy(
        "live-region",
        &[(utility_core::live_region::Part::Root, "root")],
    );
}
