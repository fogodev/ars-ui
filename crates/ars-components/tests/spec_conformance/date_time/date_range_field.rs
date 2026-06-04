use ars_components::date_time::date_range_field;

use super::assert_anatomy;

#[test]
fn date_range_field_anatomy_matches_spec() {
    assert_anatomy(
        "date-range-field",
        &[
            (date_range_field::Part::Root, "root"),
            (date_range_field::Part::Label, "label"),
            (date_range_field::Part::StartField, "start-field"),
            (date_range_field::Part::Separator, "separator"),
            (date_range_field::Part::EndField, "end-field"),
            (date_range_field::Part::Description, "description"),
            (date_range_field::Part::ErrorMessage, "error-message"),
            (date_range_field::Part::HiddenInput, "hidden-input"),
        ],
    );
}
