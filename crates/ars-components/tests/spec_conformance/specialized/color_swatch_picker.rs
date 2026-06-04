use super::{assert_anatomy, specialized_core};

#[test]
fn color_swatch_picker_anatomy_matches_spec() {
    assert_anatomy(
        "color-swatch-picker",
        &[
            (specialized_core::color_swatch_picker::Part::Root, "root"),
            (
                specialized_core::color_swatch_picker::Part::Item { index: 0 },
                "item",
            ),
            (
                specialized_core::color_swatch_picker::Part::HiddenInput,
                "hidden-input",
            ),
        ],
    );
}
