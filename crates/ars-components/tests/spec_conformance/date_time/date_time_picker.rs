use ars_components::date_time::{date_field::DateSegmentKind, date_time_picker};

use super::assert_anatomy;

#[test]
fn date_time_picker_anatomy_matches_spec() {
    assert_anatomy(
        "date-time-picker",
        &[
            (date_time_picker::Part::Root, "root"),
            (date_time_picker::Part::Label, "label"),
            (date_time_picker::Part::Control, "control"),
            (
                date_time_picker::Part::DateSegmentGroup,
                "date-segment-group",
            ),
            (
                date_time_picker::Part::TimeSegmentGroup,
                "time-segment-group",
            ),
            (
                date_time_picker::Part::Segment {
                    kind: DateSegmentKind::Year,
                },
                "segment",
            ),
            (date_time_picker::Part::Literal { index: 0 }, "literal"),
            (date_time_picker::Part::Separator, "separator"),
            (date_time_picker::Part::Trigger, "trigger"),
            (date_time_picker::Part::ClearTrigger, "clear-trigger"),
            (date_time_picker::Part::Positioner, "positioner"),
            (date_time_picker::Part::Content, "content"),
            (date_time_picker::Part::Description, "description"),
            (date_time_picker::Part::ErrorMessage, "error-message"),
            (date_time_picker::Part::HiddenInput, "hidden-input"),
        ],
    );
}
