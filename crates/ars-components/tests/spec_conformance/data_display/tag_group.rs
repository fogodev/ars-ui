use ars_components::data_display::tag_group;

use super::*;

#[test]
fn tag_group_anatomy_matches_spec() {
    assert_anatomy(
        "tag-group",
        &[
            (tag_group::Part::Root, "root"),
            (tag_group::Part::Label, "label"),
            (tag_group::Part::List, "list"),
            (
                tag_group::Part::Tag {
                    key: Key::default(),
                },
                "tag",
            ),
            (
                tag_group::Part::TagRemove {
                    key: Key::default(),
                },
                "tag-remove",
            ),
        ],
    );
}
