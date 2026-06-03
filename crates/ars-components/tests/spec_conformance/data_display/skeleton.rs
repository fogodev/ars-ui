use ars_components::data_display::skeleton;

use super::*;

#[test]
fn skeleton_anatomy_matches_spec() {
    assert_anatomy(
        "skeleton",
        &[
            (skeleton::Part::Root, "root"),
            (skeleton::Part::Circle, "circle"),
            (skeleton::Part::Item { index: 0 }, "item"),
        ],
    );
}
