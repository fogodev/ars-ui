use super::*;

#[test]
fn field_anatomy_matches_spec() {
    assert_anatomy(
        "field",
        &[
            (utility_core::field::Part::Root, "root"),
            (utility_core::field::Part::Label, "label"),
            (utility_core::field::Part::Input, "input"),
            (utility_core::field::Part::Description, "description"),
            (utility_core::field::Part::ErrorMessage, "error-message"),
        ],
    );
}
