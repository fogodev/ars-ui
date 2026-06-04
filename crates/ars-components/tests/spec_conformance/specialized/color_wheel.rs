use super::{assert_anatomy, specialized_core};

#[test]
fn color_wheel_anatomy_matches_spec() {
    assert_anatomy(
        "color-wheel",
        &[
            (specialized_core::color_wheel::Part::Root, "root"),
            (specialized_core::color_wheel::Part::Track, "track"),
            (specialized_core::color_wheel::Part::Thumb, "thumb"),
            (
                specialized_core::color_wheel::Part::HiddenInput,
                "hidden-input",
            ),
        ],
    );
}
