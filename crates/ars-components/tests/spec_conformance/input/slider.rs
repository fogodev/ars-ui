use ars_components::input::slider;

use crate::helper::assert_anatomy;

#[test]
fn slider_anatomy_matches_spec() {
    assert_anatomy(
        "slider",
        &[
            (slider::Part::Root, "root"),
            (slider::Part::Label, "label"),
            (slider::Part::Track, "track"),
            (slider::Part::Range, "range"),
            (slider::Part::Thumb, "thumb"),
            (slider::Part::Output, "output"),
            (slider::Part::MarkerGroup, "marker-group"),
            (slider::Part::Marker { value: 0.0 }, "marker"),
            (slider::Part::HiddenInput, "hidden-input"),
            (slider::Part::DraggingIndicator, "dragging-indicator"),
            (slider::Part::Description, "description"),
            (slider::Part::ErrorMessage, "error-message"),
        ],
    );
}
