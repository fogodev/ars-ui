use ars_components::layout::toolbar;

use super::*;

#[test]
fn toolbar_anatomy_matches_spec() {
    assert_anatomy(
        "toolbar",
        &[
            (toolbar::Part::Root, "root"),
            (toolbar::Part::Item { index: 0 }, "item"),
            (toolbar::Part::Separator, "separator"),
        ],
    );
}
