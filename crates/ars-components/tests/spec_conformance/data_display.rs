//! Spec-conformance tests for `crates/ars-components/src/data_display/*`.
//!
//! Asserts the impl's `Part` enum matches the spec's declared anatomy.

use ars_collections::Key;
use ars_components::data_display::{
    grid_list, marquee, meter, progress, rating_group, stat, table, tag_group,
};

use super::helper::assert_anatomy;

#[test]
fn table_anatomy_matches_spec() {
    // Spec references:
    // - `spec/components/data-display/table.md` §2.1 declares the
    //   base anatomy (Root, Table, Caption, Head, Body, Foot, Row,
    //   ColumnHeader, RowHeader, Cell, SelectAllCheckbox, RowCheckbox,
    //   ExpandTrigger, ExpandedContent).
    // - §6.3 layers the `ColumnResizeHandle` part on top for the
    //   Column Resizing variant.
    //
    // Workspace convention (see `tests/spec_conformance/helper.rs`)
    // requires the first variant to be `Root`. The remaining variants
    // appear in §2.1 declaration order, with `ColumnResizeHandle`
    // appended last because §6 layers it on the base anatomy.
    assert_anatomy(
        "table",
        &[
            (table::Part::Root, "root"),
            (table::Part::Table, "table"),
            (table::Part::Caption, "caption"),
            (table::Part::Head, "head"),
            (table::Part::Body, "body"),
            (table::Part::Foot, "foot"),
            (
                table::Part::Row {
                    key: Key::default(),
                },
                "row",
            ),
            (
                table::Part::ColumnHeader {
                    header: String::new(),
                    sortable: false,
                },
                "column-header",
            ),
            (table::Part::RowHeader, "row-header"),
            (table::Part::Cell { col: 0, row: 0 }, "cell"),
            (table::Part::SelectAllCheckbox, "select-all-checkbox"),
            (
                table::Part::RowCheckbox {
                    key: Key::default(),
                },
                "row-checkbox",
            ),
            (
                table::Part::ExpandTrigger {
                    key: Key::default(),
                },
                "expand-trigger",
            ),
            (
                table::Part::ExpandedContent {
                    key: Key::default(),
                },
                "expanded-content",
            ),
            (
                table::Part::ColumnResizeHandle {
                    column: String::new(),
                },
                "column-resize-handle",
            ),
        ],
    );
}

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

#[test]
fn progress_anatomy_matches_spec() {
    assert_anatomy(
        "progress",
        &[
            (progress::Part::Root, "root"),
            (progress::Part::Label, "label"),
            (progress::Part::Track, "track"),
            (progress::Part::Range, "range"),
            (progress::Part::ValueText, "value-text"),
            (progress::Part::CircleTrack, "circle-track"),
            (progress::Part::CircleRange { radius: 10.0 }, "circle-range"),
        ],
    );
}

#[test]
fn marquee_anatomy_matches_spec() {
    assert_anatomy(
        "marquee",
        &[
            (marquee::Part::Root, "root"),
            (marquee::Part::Content, "content"),
            (
                marquee::Part::Edge {
                    side: marquee::EdgeSide::Start,
                },
                "edge",
            ),
            (marquee::Part::AutoPlayTrigger, "auto-play-trigger"),
        ],
    );
}

#[test]
fn rating_group_anatomy_matches_spec() {
    assert_anatomy(
        "rating-group",
        &[
            (rating_group::Part::Root, "root"),
            (rating_group::Part::Label, "label"),
            (rating_group::Part::Control, "control"),
            (rating_group::Part::Item { index: 0 }, "item"),
            (rating_group::Part::HiddenInput, "hidden-input"),
        ],
    );
}

#[test]
fn tag_group_anatomy_matches_spec() {
    assert_anatomy(
        "tag-group",
        &[
            (tag_group::Part::Root, "root"),
            (tag_group::Part::Label, "label"),
            (tag_group::Part::List, "list"),
            (
                tag_group::Part::Tag {
                    key: Key::default(),
                },
                "tag",
            ),
            (
                tag_group::Part::TagRemove {
                    key: Key::default(),
                },
                "tag-remove",
            ),
        ],
    );
}

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
