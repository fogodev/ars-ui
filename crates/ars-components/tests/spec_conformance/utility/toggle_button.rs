use super::*;

#[test]
fn toggle_button_anatomy_matches_spec() {
    assert_anatomy(
        "toggle-button",
        &[(utility_core::toggle_button::Part::Root, "root")],
    );
}
