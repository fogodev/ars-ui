use ars_components::input::checkbox;

use crate::helper::assert_anatomy;

#[test]
fn checkbox_anatomy_matches_spec() {
    assert_anatomy(
        "checkbox",
        &[
            (checkbox::Part::Root, "root"),
            (checkbox::Part::Label, "label"),
            (checkbox::Part::Control, "control"),
            (checkbox::Part::Indicator, "indicator"),
            (checkbox::Part::HiddenInput, "hidden-input"),
            (checkbox::Part::Description, "description"),
            (checkbox::Part::ErrorMessage, "error-message"),
        ],
    );
}
