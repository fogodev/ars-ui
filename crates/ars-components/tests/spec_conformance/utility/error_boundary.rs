use super::*;

#[test]
fn error_boundary_anatomy_matches_spec() {
    assert_anatomy(
        "error-boundary",
        &[
            (utility_core::error_boundary::Part::Root, "root"),
            (utility_core::error_boundary::Part::Message, "message"),
            (utility_core::error_boundary::Part::List, "list"),
            (utility_core::error_boundary::Part::Item, "item"),
        ],
    );
}
