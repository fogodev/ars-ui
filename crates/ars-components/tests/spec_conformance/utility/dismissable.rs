use super::*;

#[test]
fn dismissable_anatomy_matches_spec() {
    assert_anatomy(
        "dismissable",
        &[
            (utility_core::dismissable::Part::Root, "root"),
            (
                utility_core::dismissable::Part::DismissButton,
                "dismiss-button",
            ),
        ],
    );
}
