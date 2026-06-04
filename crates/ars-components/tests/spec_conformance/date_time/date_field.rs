use ars_components::date_time::date_field::{self, DateSegmentKind};

use super::assert_anatomy;

#[test]
fn date_field_anatomy_matches_spec() {
    assert_anatomy(
        "date-field",
        &[
            (date_field::Part::Root, "root"),
            (date_field::Part::Label, "label"),
            (date_field::Part::FieldGroup, "field-group"),
            (
                date_field::Part::Segment {
                    kind: DateSegmentKind::Year,
                },
                "segment",
            ),
            (date_field::Part::Literal { index: 0 }, "literal"),
            (date_field::Part::Description, "description"),
            (date_field::Part::ErrorMessage, "error-message"),
            (date_field::Part::HiddenInput, "hidden-input"),
        ],
    );
}
