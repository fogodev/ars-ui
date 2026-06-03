use ars_components::data_display::progress;

use super::*;

#[test]
fn progress_anatomy_matches_spec() {
    assert_anatomy(
        "progress",
        &[
            (progress::Part::Root, "root"),
            (progress::Part::Label, "label"),
            (progress::Part::Track, "track"),
            (progress::Part::Range, "range"),
            (progress::Part::ValueText, "value-text"),
            (progress::Part::CircleTrack, "circle-track"),
            (progress::Part::CircleRange { radius: 10.0 }, "circle-range"),
        ],
    );
}
