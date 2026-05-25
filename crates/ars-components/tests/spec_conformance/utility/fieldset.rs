use super::*;

#[test]
fn fieldset_anatomy_matches_spec() {
    assert_anatomy(
        "fieldset",
        &[
            (utility_core::fieldset::Part::Root, "root"),
            (utility_core::fieldset::Part::Legend, "legend"),
            (utility_core::fieldset::Part::Description, "description"),
            (utility_core::fieldset::Part::ErrorMessage, "error-message"),
            (utility_core::fieldset::Part::Content, "content"),
        ],
    );
}
