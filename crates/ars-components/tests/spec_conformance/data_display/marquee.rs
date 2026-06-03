use ars_components::data_display::marquee;

use super::*;

#[test]
fn marquee_anatomy_matches_spec() {
    assert_anatomy(
        "marquee",
        &[
            (marquee::Part::Root, "root"),
            (marquee::Part::Content, "content"),
            (
                marquee::Part::Edge {
                    side: marquee::EdgeSide::Start,
                },
                "edge",
            ),
            (marquee::Part::AutoPlayTrigger, "auto-play-trigger"),
        ],
    );
}
