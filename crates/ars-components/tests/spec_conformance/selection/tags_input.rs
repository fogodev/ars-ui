use ars_components::selection::tags_input;

use super::assert_anatomy;

#[test]
fn tags_input_anatomy_matches_spec() {
    assert_anatomy(
        "tags-input",
        &[
            (tags_input::Part::Root, "root"),
            (tags_input::Part::Label, "label"),
            (tags_input::Part::Control, "control"),
            (tags_input::Part::Tag { index: 0 }, "tag"),
            (tags_input::Part::TagText { index: 0 }, "tag-text"),
            (
                tags_input::Part::TagDeleteCell { index: 0 },
                "tag-delete-cell",
            ),
            (
                tags_input::Part::TagDeleteTrigger { index: 0 },
                "tag-delete-trigger",
            ),
            (tags_input::Part::TagEdit { index: 0 }, "tag-edit"),
            (tags_input::Part::Input, "input"),
            (tags_input::Part::ClearTrigger, "clear-trigger"),
            (tags_input::Part::HiddenInput, "hidden-input"),
            (tags_input::Part::Description, "description"),
            (tags_input::Part::ErrorMessage, "error-message"),
            (tags_input::Part::LiveRegion, "live-region"),
        ],
    );
}
