use super::{assert_anatomy, specialized_core};

#[test]
fn color_area_anatomy_matches_spec() {
    assert_anatomy(
        "color-area",
        &[
            (specialized_core::color_area::Part::Root, "root"),
            (specialized_core::color_area::Part::Background, "background"),
            (specialized_core::color_area::Part::Thumb, "thumb"),
            (
                specialized_core::color_area::Part::HiddenInput,
                "hidden-input",
            ),
        ],
    );
}
