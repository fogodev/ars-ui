use ars_components::input::range_slider;

use crate::helper::assert_anatomy;

#[test]
fn range_slider_anatomy_matches_spec() {
    assert_anatomy(
        "range-slider",
        &[
            (range_slider::Part::Root, "root"),
            (range_slider::Part::Label, "label"),
            (range_slider::Part::Track, "track"),
            (range_slider::Part::Range, "range"),
            (
                range_slider::Part::Thumb {
                    thumb: range_slider::ThumbIndex::Start,
                },
                "thumb",
            ),
            (range_slider::Part::Output, "output"),
            (range_slider::Part::MarkerGroup, "marker-group"),
            (range_slider::Part::Marker { value: 0.0 }, "marker"),
            (
                range_slider::Part::HiddenInput {
                    thumb: range_slider::ThumbIndex::Start,
                },
                "hidden-input",
            ),
            (range_slider::Part::DraggingIndicator, "dragging-indicator"),
            (range_slider::Part::Description, "description"),
            (range_slider::Part::ErrorMessage, "error-message"),
        ],
    );
}
