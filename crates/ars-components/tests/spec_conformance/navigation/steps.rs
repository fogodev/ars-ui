use ars_components::navigation::steps;

use crate::helper::assert_anatomy;

#[test]
fn steps_anatomy_matches_spec() {
    assert_anatomy(
        "steps",
        &[
            (steps::Part::Root, "root"),
            (steps::Part::List, "list"),
            (steps::Part::Item { index: 0 }, "item"),
            (steps::Part::Indicator { index: 0 }, "indicator"),
            (steps::Part::Title { index: 0 }, "title"),
            (steps::Part::Description { index: 0 }, "description"),
            (steps::Part::Separator { after_index: 0 }, "separator"),
            (steps::Part::Content { index: 0 }, "content"),
            (steps::Part::PrevTrigger, "prev-trigger"),
            (steps::Part::NextTrigger, "next-trigger"),
        ],
    );
}
