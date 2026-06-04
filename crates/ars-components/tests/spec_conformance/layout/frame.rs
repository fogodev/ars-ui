use ars_components::layout::frame;

use super::*;

#[test]
fn frame_anatomy_matches_spec() {
    assert_anatomy(
        "frame",
        &[(frame::Part::Root, "root"), (frame::Part::Iframe, "iframe")],
    );
}
