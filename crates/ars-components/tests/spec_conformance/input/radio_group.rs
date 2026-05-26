use ars_components::input::radio_group;

use crate::helper::assert_anatomy;

#[test]
fn radio_group_anatomy_matches_spec() {
    assert_anatomy(
        "radio-group",
        &[
            (radio_group::Part::Root, "root"),
            (radio_group::Part::Label, "label"),
            (
                radio_group::Part::Item {
                    item_value: Default::default(),
                },
                "item",
            ),
            (
                radio_group::Part::ItemControl {
                    item_value: Default::default(),
                },
                "item-control",
            ),
            (
                radio_group::Part::ItemIndicator {
                    item_value: Default::default(),
                },
                "item-indicator",
            ),
            (
                radio_group::Part::ItemLabel {
                    item_value: Default::default(),
                },
                "item-label",
            ),
            (
                radio_group::Part::ItemHiddenInput {
                    item_value: Default::default(),
                },
                "item-hidden-input",
            ),
            (radio_group::Part::Description, "description"),
            (radio_group::Part::ErrorMessage, "error-message"),
        ],
    );
}
