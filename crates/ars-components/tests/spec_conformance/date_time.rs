//! Spec-conformance tests for `crates/ars-components/src/date_time/*`.
//!
//! Asserts each date-time component's `Part` enum matches the spec anatomy.

use ars_components::date_time::{
    calendar, date_field::DateSegmentKind, date_picker, range_calendar, time_field,
};
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
                    offset: 0,
                },
                "cell",
            ),
            (
                calendar::Part::CellTrigger {
                    date: example.clone(),
                    offset: 0,
                },
                "cell-trigger",
            ),
        ],
    );
}

#[test]
fn range_calendar_anatomy_matches_spec() {
    let example = calendar_date_default_for_part_all();
    assert_anatomy(
        "range-calendar",
        &[
            (range_calendar::Part::Root, "root"),
            (range_calendar::Part::Header, "header"),
            (range_calendar::Part::PrevTrigger, "prev-trigger"),
            (range_calendar::Part::NextTrigger, "next-trigger"),
            (range_calendar::Part::Heading, "heading"),
            (range_calendar::Part::Grid, "grid"),
            (range_calendar::Part::GridGroup, "grid-group"),
            (range_calendar::Part::HeadRow, "head-row"),
            (
                range_calendar::Part::HeadCell {
                    day: Weekday::Monday,
                },
                "head-cell",
            ),
            (range_calendar::Part::Row { week_index: 0 }, "row"),
            (
                range_calendar::Part::Cell {
                    date: example.clone(),
                    offset: 0,
                },
                "cell",
            ),
            (
                range_calendar::Part::CellTrigger {
                    date: example,
                    offset: 0,
                },
                "cell-trigger",
            ),
        ],
    );
}

#[test]
fn date_picker_anatomy_matches_spec() {
    assert_anatomy(
        "date-picker",
        &[
            (date_picker::Part::Root, "root"),
            (date_picker::Part::Label, "label"),
            (date_picker::Part::Control, "control"),
            (date_picker::Part::Input, "input"),
            (date_picker::Part::Trigger, "trigger"),
            (date_picker::Part::ClearTrigger, "clear-trigger"),
            (date_picker::Part::Positioner, "positioner"),
            (date_picker::Part::Content, "content"),
            (date_picker::Part::Description, "description"),
            (date_picker::Part::ErrorMessage, "error-message"),
            (date_picker::Part::HiddenInput, "hidden-input"),
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
