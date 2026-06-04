use super::{assert_anatomy, specialized_core};

#[test]
fn angle_slider_anatomy_matches_spec() {
    assert_anatomy(
        "angle-slider",
        &[
            (specialized_core::angle_slider::Part::Root, "root"),
            (specialized_core::angle_slider::Part::Control, "control"),
            (specialized_core::angle_slider::Part::Track, "track"),
            (specialized_core::angle_slider::Part::Range, "range"),
            (specialized_core::angle_slider::Part::Thumb, "thumb"),
            (
                specialized_core::angle_slider::Part::ValueText,
                "value-text",
            ),
            (
                specialized_core::angle_slider::Part::MarkerGroup,
                "marker-group",
            ),
            (
                specialized_core::angle_slider::Part::Marker { value: 0.0 },
                "marker",
            ),
            (
                specialized_core::angle_slider::Part::HiddenInput,
                "hidden-input",
            ),
        ],
    );
}
