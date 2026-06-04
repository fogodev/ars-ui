use ars_components::date_time::calendar;
use ars_i18n::Weekday;

use super::{assert_anatomy, calendar_date_default_for_part_all};

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
                    date: example,
                    offset: 0,
                },
                "cell-trigger",
            ),
        ],
    );
}
