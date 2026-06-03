use ars_components::data_display::avatar;

use super::*;

#[test]
fn avatar_anatomy_matches_spec() {
    assert_anatomy(
        "avatar",
        &[
            (avatar::Part::Root, "root"),
            (avatar::Part::Image, "image"),
            (avatar::Part::Fallback, "fallback"),
        ],
    );
}

#[test]
fn avatar_group_anatomy_matches_spec() {
    assert_anatomy(
        "avatar",
        &[
            (avatar::GroupPart::Group, "group"),
            (avatar::GroupPart::GroupItem { index: 0 }, "group-item"),
        ],
    );
}
