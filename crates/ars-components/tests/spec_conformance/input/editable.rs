use ars_components::input::editable;

use crate::helper::assert_anatomy;

#[test]
fn editable_anatomy_matches_spec() {
    assert_anatomy(
        "editable",
        &[
            (editable::Part::Root, "root"),
            (editable::Part::Label, "label"),
            (editable::Part::Preview, "preview"),
            (editable::Part::Input, "input"),
            (editable::Part::EditTrigger, "edit-trigger"),
            (editable::Part::SubmitTrigger, "submit-trigger"),
            (editable::Part::CancelTrigger, "cancel-trigger"),
        ],
    );
}
