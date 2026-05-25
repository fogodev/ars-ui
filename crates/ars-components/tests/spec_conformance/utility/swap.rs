use super::*;

#[test]
fn swap_anatomy_matches_spec() {
    assert_anatomy(
        "swap",
        &[
            (utility_core::swap::Part::Root, "root"),
            (utility_core::swap::Part::OnContent, "on-content"),
            (utility_core::swap::Part::OffContent, "off-content"),
        ],
    );
}
