use ars_components::data_display::badge;

use super::*;

#[test]
fn badge_anatomy_matches_spec() {
    assert_anatomy("badge", &[(badge::Part::Root, "root")]);
}
