use ars_components::input::checkbox_group;

use crate::helper::assert_anatomy;

#[test]
fn checkbox_group_anatomy_matches_spec() {
    assert_anatomy(
        "checkbox-group",
        &[
            (checkbox_group::Part::Root, "root"),
            (checkbox_group::Part::Label, "label"),
            (checkbox_group::Part::Description, "description"),
            (checkbox_group::Part::ErrorMessage, "error-message"),
        ],
    );
}
