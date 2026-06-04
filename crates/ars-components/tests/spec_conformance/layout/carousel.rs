use ars_components::layout::carousel;

use super::*;

#[test]
fn carousel_anatomy_matches_spec() {
    assert_anatomy(
        "carousel",
        &[
            (carousel::Part::Root, "root"),
            (carousel::Part::Viewport, "viewport"),
            (carousel::Part::ItemGroup, "item-group"),
            (carousel::Part::Item { index: 0 }, "item"),
            (carousel::Part::PrevTrigger, "prev-trigger"),
            (carousel::Part::NextTrigger, "next-trigger"),
            (carousel::Part::IndicatorGroup, "indicator-group"),
            (carousel::Part::Indicator { index: 0 }, "indicator"),
            (carousel::Part::AutoPlayTrigger, "auto-play-trigger"),
            (carousel::Part::AutoPlayIndicator, "auto-play-indicator"),
            (carousel::Part::ProgressText, "progress-text"),
        ],
    );
}
