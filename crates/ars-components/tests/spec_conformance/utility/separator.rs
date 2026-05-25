use super::*;

#[test]
fn separator_anatomy_matches_spec() {
    assert_anatomy(
        "separator",
        &[(utility_core::separator::Part::Root, "root")],
    );
}
