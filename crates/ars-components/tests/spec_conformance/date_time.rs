//! Spec-conformance tests for `crates/ars-components/src/date_time/*`.
//!
//! Asserts each date-time component's `Part` enum matches the spec anatomy.

use ars_components::date_time::{date_field::DateSegmentKind, time_field};

use super::helper::assert_anatomy;

#[test]
fn time_field_anatomy_matches_spec() {
    assert_anatomy(
        "time-field",
        &[
            (time_field::Part::Root, "root"),
            (time_field::Part::Label, "label"),
            (time_field::Part::FieldGroup, "field-group"),
            (
                time_field::Part::Segment {
                    kind: DateSegmentKind::Year,
                },
                "segment",
            ),
            (time_field::Part::Literal { index: 0 }, "literal"),
            (time_field::Part::Description, "description"),
            (time_field::Part::ErrorMessage, "error-message"),
            (time_field::Part::HiddenInput, "hidden-input"),
        ],
    );
}
