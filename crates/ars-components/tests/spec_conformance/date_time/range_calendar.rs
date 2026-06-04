use ars_components::date_time::range_calendar;
use ars_i18n::Weekday;

use super::{assert_anatomy, calendar_date_default_for_part_all};

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
