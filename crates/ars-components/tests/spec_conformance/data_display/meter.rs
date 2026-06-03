use ars_components::data_display::meter;

use super::*;

#[test]
fn meter_anatomy_matches_spec() {
    assert_anatomy(
        "meter",
        &[
            (meter::Part::Root, "root"),
            (meter::Part::Label, "label"),
            (meter::Part::Track, "track"),
            (meter::Part::Range, "range"),
            (meter::Part::ValueText, "value-text"),
        ],
    );
}
