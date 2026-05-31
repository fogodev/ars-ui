use ars_components::navigation::link;

use crate::helper::assert_anatomy;

#[test]
fn link_anatomy_matches_spec() {
    assert_anatomy("link", &[(link::Part::Root, "root")]);
}
