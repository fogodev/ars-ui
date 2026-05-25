use super::*;

#[test]
fn action_group_anatomy_matches_spec() {
    assert_anatomy(
        "action-group",
        &[
            (utility_core::action_group::Part::Root, "root"),
            (
                utility_core::action_group::Part::Item {
                    item_id: Key::default(),
                },
                "item",
            ),
            (
                utility_core::action_group::Part::OverflowTrigger,
                "overflow-trigger",
            ),
        ],
    );
}
