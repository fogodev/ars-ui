use ars_components::layout::center;

use super::*;

#[test]
fn center_anatomy_matches_spec() {
    assert_anatomy("center", &[(center::Part::Root, "root")]);
}
