use super::{assert_anatomy, specialized_core};

#[test]
fn color_swatch_anatomy_matches_spec() {
    assert_anatomy(
        "color-swatch",
        &[
            (specialized_core::color_swatch::Part::Root, "root"),
            (specialized_core::color_swatch::Part::Inner, "inner"),
        ],
    );
}
