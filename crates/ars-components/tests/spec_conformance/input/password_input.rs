use ars_components::input::password_input;

use crate::helper::assert_anatomy;

#[test]
fn password_input_anatomy_matches_spec() {
    assert_anatomy(
        "password-input",
        &[
            (password_input::Part::Root, "root"),
            (password_input::Part::Label, "label"),
            (password_input::Part::Input, "input"),
            (password_input::Part::Toggle, "toggle"),
            (password_input::Part::Description, "description"),
            (password_input::Part::ErrorMessage, "error-message"),
        ],
    );
}
