use super::*;

#[test]
fn form_submit_anatomy_matches_spec() {
    assert_anatomy(
        "form-submit",
        &[
            (utility_core::form_submit::Part::Root, "root"),
            (
                utility_core::form_submit::Part::SubmitButton,
                "submit-button",
            ),
        ],
    );
}
