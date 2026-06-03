use ars_components::data_display::grid_list;

use super::*;

#[test]
fn grid_list_anatomy_matches_spec() {
    assert_anatomy(
        "grid-list",
        &[
            (grid_list::Part::Root, "root"),
            (
                grid_list::Part::Row {
                    key: Key::default(),
                },
                "row",
            ),
            (
                grid_list::Part::Cell {
                    key: Key::default(),
                },
                "cell",
            ),
            (grid_list::Part::LoadingSentinel, "loading-sentinel"),
            (
                grid_list::Part::DragHandle {
                    key: Key::default(),
                },
                "drag-handle",
            ),
            (grid_list::Part::DropIndicator, "drop-indicator"),
        ],
    );
}
