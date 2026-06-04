use ars_components::layout::splitter;

use super::*;

#[test]
fn splitter_anatomy_matches_spec() {
    assert_anatomy(
        "splitter",
        &[
            (splitter::Part::Root, "root"),
            (splitter::Part::Panel { index: 0 }, "panel"),
            (splitter::Part::Handle { index: 0 }, "handle"),
        ],
    );
}
