use ars_components::layout::aspect_ratio;

use super::*;

#[test]
fn aspect_ratio_anatomy_matches_spec() {
    assert_anatomy("aspect-ratio", &[(aspect_ratio::Part::Root, "root")]);
}
