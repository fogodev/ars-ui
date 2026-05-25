use super::*;

#[test]
fn visually_hidden_anatomy_matches_spec() {
    assert_anatomy(
        "visually-hidden",
        &[(utility_core::visually_hidden::Part::Root, "root")],
    );
}
