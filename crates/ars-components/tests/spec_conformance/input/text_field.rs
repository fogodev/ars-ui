use ars_components::input::text_field;

use crate::helper::assert_anatomy;

#[test]
fn text_field_anatomy_matches_spec() {
    assert_anatomy(
        "text-field",
        &[
            (text_field::Part::Root, "root"),
            (text_field::Part::Label, "label"),
            (text_field::Part::Input, "input"),
            (text_field::Part::StartDecorator, "start-decorator"),
            (text_field::Part::EndDecorator, "end-decorator"),
            (text_field::Part::ClearTrigger, "clear-trigger"),
            (text_field::Part::Description, "description"),
            (text_field::Part::ErrorMessage, "error-message"),
        ],
    );
}
