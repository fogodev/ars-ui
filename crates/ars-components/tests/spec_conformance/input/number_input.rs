use ars_components::input::number_input;

use crate::helper::assert_anatomy;

#[test]
fn number_input_anatomy_matches_spec() {
    assert_anatomy(
        "number-input",
        &[
            (number_input::Part::Root, "root"),
            (number_input::Part::Label, "label"),
            (number_input::Part::Input, "input"),
            (number_input::Part::IncrementTrigger, "increment-trigger"),
            (number_input::Part::DecrementTrigger, "decrement-trigger"),
            (number_input::Part::Description, "description"),
            (number_input::Part::ErrorMessage, "error-message"),
        ],
    );
}
