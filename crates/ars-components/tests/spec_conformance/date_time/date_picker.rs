use ars_components::date_time::date_picker;

use super::assert_anatomy;

#[test]
fn date_picker_anatomy_matches_spec() {
    assert_anatomy(
        "date-picker",
        &[
            (date_picker::Part::Root, "root"),
            (date_picker::Part::Label, "label"),
            (date_picker::Part::Control, "control"),
            (date_picker::Part::Input, "input"),
            (date_picker::Part::Trigger, "trigger"),
            (date_picker::Part::ClearTrigger, "clear-trigger"),
            (date_picker::Part::Positioner, "positioner"),
            (date_picker::Part::Content, "content"),
            (date_picker::Part::Description, "description"),
            (date_picker::Part::ErrorMessage, "error-message"),
            (date_picker::Part::HiddenInput, "hidden-input"),
        ],
    );
}
