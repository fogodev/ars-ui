use super::{assert_anatomy, specialized_core};

#[test]
fn color_field_anatomy_matches_spec() {
    assert_anatomy(
        "color-field",
        &[
            (specialized_core::color_field::Part::Root, "root"),
            (specialized_core::color_field::Part::Label, "label"),
            (specialized_core::color_field::Part::Input, "input"),
            (
                specialized_core::color_field::Part::Description,
                "description",
            ),
            (
                specialized_core::color_field::Part::ErrorMessage,
                "error-message",
            ),
            (
                specialized_core::color_field::Part::HiddenInput,
                "hidden-input",
            ),
        ],
    );
}
