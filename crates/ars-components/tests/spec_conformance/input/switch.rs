use ars_components::input::switch;

use crate::helper::assert_anatomy;

#[test]
fn switch_anatomy_matches_spec() {
    assert_anatomy(
        "switch",
        &[
            (switch::Part::Root, "root"),
            (switch::Part::Label, "label"),
            (switch::Part::Control, "control"),
            (switch::Part::Thumb, "thumb"),
            (switch::Part::HiddenInput, "hidden-input"),
            (switch::Part::Description, "description"),
            (switch::Part::ErrorMessage, "error-message"),
        ],
    );
}
