use ars_components::layout::portal;

use super::*;

#[test]
fn portal_anatomy_matches_spec() {
    assert_anatomy("portal", &[(portal::Part::Root, "root")]);
}
