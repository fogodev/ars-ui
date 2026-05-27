use ars_components::input::file_trigger;

use crate::helper::assert_anatomy;

#[test]
fn file_trigger_anatomy_matches_spec() {
    assert_anatomy(
        "file-trigger",
        &[
            (file_trigger::Part::Root, "root"),
            (file_trigger::Part::Trigger, "trigger"),
            (file_trigger::Part::Input, "input"),
        ],
    );
}
