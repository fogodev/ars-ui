use super::*;

#[test]
fn button_anatomy_matches_spec() {
    assert_anatomy(
        "button",
        &[
            (utility_core::button::Part::Root, "root"),
            (
                utility_core::button::Part::LoadingIndicator,
                "loading-indicator",
            ),
            (utility_core::button::Part::Content, "content"),
        ],
    );
}
