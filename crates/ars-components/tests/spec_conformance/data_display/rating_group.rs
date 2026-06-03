use ars_components::data_display::rating_group;

use super::*;

#[test]
fn rating_group_anatomy_matches_spec() {
    assert_anatomy(
        "rating-group",
        &[
            (rating_group::Part::Root, "root"),
            (rating_group::Part::Label, "label"),
            (rating_group::Part::Control, "control"),
            (rating_group::Part::Item { index: 0 }, "item"),
            (rating_group::Part::HiddenInput, "hidden-input"),
        ],
    );
}
