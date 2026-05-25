use ars_components::overlay::alert_dialog;

use crate::helper::assert_anatomy;

#[test]
fn alert_dialog_anatomy_matches_spec() {
    assert_anatomy(
        "alert-dialog",
        &[
            (alert_dialog::Part::Root, "root"),
            (alert_dialog::Part::Trigger, "trigger"),
            (alert_dialog::Part::Backdrop, "backdrop"),
            (alert_dialog::Part::Positioner, "positioner"),
            (alert_dialog::Part::Content, "content"),
            (alert_dialog::Part::Title, "title"),
            (alert_dialog::Part::Description, "description"),
            (alert_dialog::Part::CancelTrigger, "cancel-trigger"),
            (alert_dialog::Part::ActionTrigger, "action-trigger"),
            (alert_dialog::Part::CloseTrigger, "close-trigger"),
        ],
    );
}
