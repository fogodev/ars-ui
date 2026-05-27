use ars_components::input::pin_input;

use crate::helper::assert_anatomy;

#[test]
fn pin_input_anatomy_matches_spec() {
    assert_anatomy(
        "pin-input",
        &[
            (pin_input::Part::Root, "root"),
            (pin_input::Part::Label, "label"),
            (pin_input::Part::Input { cell_index: 0 }, "input"),
            (pin_input::Part::HiddenInput, "hidden-input"),
            (pin_input::Part::Description, "description"),
            (pin_input::Part::ErrorMessage, "error-message"),
        ],
    );
}
