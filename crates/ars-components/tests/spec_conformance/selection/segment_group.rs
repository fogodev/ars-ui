use ars_components::selection::segment_group;

use super::{Key, assert_anatomy};

#[test]
fn segment_group_anatomy_matches_spec() {
    assert_anatomy(
        "segment-group",
        &[
            (segment_group::Part::Root, "root"),
            (
                segment_group::Part::Item {
                    value: Key::default(),
                },
                "item",
            ),
            (
                segment_group::Part::ItemText {
                    value: Key::default(),
                },
                "item-text",
            ),
            (segment_group::Part::Indicator, "indicator"),
            (segment_group::Part::HiddenInput, "hidden-input"),
        ],
    );
}
