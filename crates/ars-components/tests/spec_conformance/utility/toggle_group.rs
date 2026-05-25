use super::*;

#[test]
fn toggle_group_anatomy_matches_spec() {
    assert_anatomy(
        "toggle-group",
        &[
            (utility_core::toggle_group::Part::Root, "root"),
            (
                utility_core::toggle_group::Part::Item {
                    value: Key::default(),
                },
                "item",
            ),
            (utility_core::toggle_group::Part::Indicator, "indicator"),
        ],
    );
}
