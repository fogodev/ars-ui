use super::*;

#[test]
fn toggle_anatomy_matches_spec() {
    assert_anatomy(
        "toggle",
        &[
            (utility_core::toggle::Part::Root, "root"),
            (utility_core::toggle::Part::Indicator, "indicator"),
        ],
    );
}
