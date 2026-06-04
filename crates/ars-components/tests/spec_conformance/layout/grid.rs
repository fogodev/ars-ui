use ars_components::layout::grid;

use super::*;

#[test]
fn grid_anatomy_matches_spec() {
    assert_anatomy("grid", &[(grid::Part::Root, "root")]);
}
