use ars_components::overlay::drawer;

use crate::helper::assert_anatomy;

#[test]
fn drawer_anatomy_matches_spec() {
    assert_anatomy(
        "drawer",
        &[
            (drawer::Part::Root, "root"),
            (drawer::Part::Trigger, "trigger"),
            (drawer::Part::Backdrop, "backdrop"),
            (drawer::Part::Positioner, "positioner"),
            (drawer::Part::Content, "content"),
            (drawer::Part::Title, "title"),
            (drawer::Part::Description, "description"),
            (drawer::Part::Header, "header"),
            (drawer::Part::Body, "body"),
            (drawer::Part::Footer, "footer"),
            (drawer::Part::CloseTrigger, "close-trigger"),
            (drawer::Part::DragHandle, "drag-handle"),
        ],
    );
}
