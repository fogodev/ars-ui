use ars_components::date_time::date_range_picker;

use super::assert_anatomy;

#[test]
fn date_range_picker_anatomy_matches_spec() {
    assert_anatomy(
        "date-range-picker",
        &[
            (date_range_picker::Part::Root, "root"),
            (date_range_picker::Part::Label, "label"),
            (date_range_picker::Part::Control, "control"),
            (date_range_picker::Part::StartInput, "start-input"),
            (date_range_picker::Part::Separator, "separator"),
            (date_range_picker::Part::EndInput, "end-input"),
            (date_range_picker::Part::Trigger, "trigger"),
            (date_range_picker::Part::ClearTrigger, "clear-trigger"),
            (
                date_range_picker::Part::PresetTrigger { index: 0 },
                "preset-trigger",
            ),
            (date_range_picker::Part::Positioner, "positioner"),
            (date_range_picker::Part::Content, "content"),
            (date_range_picker::Part::Description, "description"),
            (date_range_picker::Part::ErrorMessage, "error-message"),
            (date_range_picker::Part::HiddenInput, "hidden-input"),
        ],
    );
}
