//! Spec-conformance tests for `crates/ars-components/src/date_time/*`.
//!
//! Asserts each date-time component's `Part` enum matches the spec anatomy.

use ars_components::date_time::{calendar, date_field::DateSegmentKind, time_field};
use ars_i18n::{CalendarDate, Weekday};

use super::helper::assert_anatomy;

fn calendar_date_default_for_part_all() -> CalendarDate {
    // Mirrors the `#[part(default = ...)]` value on `calendar::Part::Cell`
    // and `::CellTrigger`. `Part::all()` constructs these variants with the
    // declared defaults, and the spec-conformance helper compares
    // `Part::all()[i]` against the supplied expected parts via `PartialEq`,
    // so the test must use the same date the derive emits.
    CalendarDate::new_gregorian(1, 1, 1).expect("0001-01-01 is a valid Gregorian date")
}

#[test]
fn calendar_anatomy_matches_spec() {
    let example = calendar_date_default_for_part_all();
    assert_anatomy(
        "calendar",
        &[
            (calendar::Part::Root, "root"),
            (calendar::Part::Header, "header"),
            (calendar::Part::PrevTrigger, "prev-trigger"),
            (calendar::Part::NextTrigger, "next-trigger"),
            (calendar::Part::Heading, "heading"),
            (calendar::Part::Grid, "grid"),
            (calendar::Part::GridGroup, "grid-group"),
            (calendar::Part::HeadRow, "head-row"),
            (
                calendar::Part::HeadCell {
                    day: Weekday::Monday,
                },
                "head-cell",
            ),
            (calendar::Part::Row { week_index: 0 }, "row"),
            (
                calendar::Part::Cell {
                    date: example.clone(),
                },
                "cell",
            ),
            (
                calendar::Part::CellTrigger {
                    date: example.clone(),
                },
                "cell-trigger",
            ),
        ],
    );
}

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
