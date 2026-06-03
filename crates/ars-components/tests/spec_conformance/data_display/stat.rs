use ars_components::data_display::stat;

use super::*;

#[test]
fn stat_anatomy_matches_spec() {
    assert_anatomy(
        "stat",
        &[
            (stat::Part::Root, "root"),
            (stat::Part::Label, "label"),
            (stat::Part::Value, "value"),
            (stat::Part::Change, "change"),
            (stat::Part::TrendIndicator, "trend-indicator"),
            (stat::Part::HelpText, "help-text"),
        ],
    );
}
