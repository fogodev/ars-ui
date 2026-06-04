use super::{assert_anatomy, specialized_core};

#[test]
fn color_slider_anatomy_matches_spec() {
    assert_anatomy(
        "color-slider",
        &[
            (specialized_core::color_slider::Part::Root, "root"),
            (specialized_core::color_slider::Part::Label, "label"),
            (specialized_core::color_slider::Part::Track, "track"),
            (specialized_core::color_slider::Part::Thumb, "thumb"),
            (specialized_core::color_slider::Part::Output, "output"),
            (
                specialized_core::color_slider::Part::HiddenInput,
                "hidden-input",
            ),
        ],
    );
}
