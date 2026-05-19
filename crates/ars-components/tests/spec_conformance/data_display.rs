//! Spec-conformance tests for `crates/ars-components/src/data_display/*`.
//!
//! Asserts the impl's `Part` enum matches the spec's declared anatomy.

use ars_collections::Key;
use ars_components::data_display::table;

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
