use super::*;

#[test]
fn form_anatomy_matches_spec() {
    assert_anatomy(
        "form",
        &[
            (utility_core::form::Part::Root, "root"),
            (utility_core::form::Part::StatusRegion, "status-region"),
        ],
    );
}
