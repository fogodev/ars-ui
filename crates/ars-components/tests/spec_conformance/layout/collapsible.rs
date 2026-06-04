use ars_components::layout::collapsible;

use super::*;

#[test]
fn collapsible_anatomy_matches_spec() {
    assert_anatomy(
        "collapsible",
        &[
            (collapsible::Part::Root, "root"),
            (collapsible::Part::Trigger, "trigger"),
            (collapsible::Part::Indicator, "indicator"),
            (collapsible::Part::Content, "content"),
        ],
    );
}
