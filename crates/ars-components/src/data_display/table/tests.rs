//! Inline unit + snapshot tests for the [`Table`](super) state machine.
//!
//! Tests are organized by topic, mirroring the issue's "Tests to add
//! first" checklist (#286) and covering the §5 `SelectAll`, §6 Column
//! Resizing, and §3.5 Virtual Scrolling variants.

use alloc::{
    collections::BTreeSet,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::cell::RefCell;

use ars_collections::{Key, selection};
use ars_core::{AriaAttr, AttrMap, Env, HtmlAttr, MessageFn, Service};
use ars_interactions::{KeyboardEventData, KeyboardKey};
use insta::assert_snapshot;

use super::*;

// ── Test fixtures ────────────────────────────────────────────────────

/// Builds a `Key::String` from a `&str`.
fn key(value: &str) -> Key {
    Key::str(value)
}

/// Builds a baseline keyboard event with the given normalized key and no
/// modifiers held.
fn keydown(k: KeyboardKey) -> KeyboardEventData {
    KeyboardEventData {
        key: k,
        character: None,
        code: String::new(),
        shift_key: false,
        ctrl_key: false,
        alt_key: false,
        meta_key: false,
        repeat: false,
        is_composing: false,
    }
}

/// Builds a Ctrl-modified keyboard event.
fn ctrl_keydown(k: KeyboardKey) -> KeyboardEventData {
    let mut data = keydown(k);

    data.ctrl_key = true;

    data
}

/// Returns baseline test props with a stable component id.
fn test_props() -> Props {
    Props {
        id: "table".to_string(),
        ..Props::default()
    }
}

/// Returns test props in multi-selection mode.
fn test_props_multi() -> Props {
    Props {
        id: "table".to_string(),
        selection_mode: selection::Mode::Multiple,
        ..Props::default()
    }
}

/// Returns test props in single-selection mode.
fn test_props_single() -> Props {
    Props {
        id: "table".to_string(),
        selection_mode: selection::Mode::Single,
        ..Props::default()
    }
}

/// Builds a service with the supplied props and registers `rows` so
/// selection / expansion pruning has data to work against.
fn service_with_rows(props: Props, rows: &[Key]) -> Service<Machine> {
    let mut service = Service::<Machine>::new(props, &Env::default(), &Messages::default());

    drop(service.send(Event::SetRows(rows.to_vec())));

    service
}

/// Records every event dispatched through the connected `send` closure.
type EventRecorder = RefCell<Vec<Event>>;

/// Pushes the captured event into the supplied recorder.
fn record(recorder: &EventRecorder, event: Event) {
    recorder.borrow_mut().push(event);
}

/// Snapshot the `AttrMap` debug representation.
fn snapshot_attrs(attrs: &AttrMap) -> String {
    format!("{attrs:#?}")
}

// ────────────────────────────────────────────────────────────────────
// 1. Root / table element role + caption labelling
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_root_produces_table_role() {
    let service = service_with_rows(test_props(), &[]);

    let attrs = service.connect(&|_| {}).table_attrs();

    assert_eq!(
        attrs.get(&HtmlAttr::Role).map(ToString::to_string),
        Some("table".to_string())
    );
    assert_snapshot!("table_table_role_non_interactive", snapshot_attrs(&attrs));
}

#[test]
fn table_root_produces_grid_role() {
    let service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[],
    );

    let attrs = service.connect(&|_| {}).table_attrs();

    assert_eq!(
        attrs.get(&HtmlAttr::Role).map(ToString::to_string),
        Some("grid".to_string())
    );
    assert_snapshot!("table_table_role_interactive_grid", snapshot_attrs(&attrs));
}

#[test]
fn table_caption_labelledby_when_caption_present() {
    let service = service_with_rows(
        Props {
            caption: Some("Order history".to_string()),
            ..test_props()
        },
        &[],
    );

    let attrs = service.connect(&|_| {}).table_attrs();

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::LabelledBy))
            .map(ToString::to_string),
        Some("table-caption".to_string()),
    );
}

#[test]
fn table_sticky_header_data_attr() {
    let service = service_with_rows(
        Props {
            sticky_header: true,
            ..test_props()
        },
        &[],
    );

    let api = service.connect(&|_| {});

    let root = api.root_attrs();

    assert!(root.get(&HtmlAttr::Data("ars-sticky-header")).is_some());

    let head = api.head_attrs();

    assert!(head.get(&HtmlAttr::Data("ars-sticky")).is_some());

    assert_snapshot!("table_root_sticky_header", snapshot_attrs(&root));
    assert_snapshot!("table_head_sticky", snapshot_attrs(&head));
}

// ────────────────────────────────────────────────────────────────────
// 2. Column headers — sortable cycle + non-sortable
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_column_header_sortable_aria_sort_none_default() {
    let service = service_with_rows(test_props(), &[]);

    let attrs = service.connect(&|_| {}).column_header_attrs("name", true);

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::Sort))
            .map(ToString::to_string),
        Some("none".to_string()),
    );
    assert_snapshot!(
        "table_column_header_sortable_unsorted",
        snapshot_attrs(&attrs)
    );
}

#[test]
fn table_column_header_sortable_aria_sort_ascending_after_click() {
    let mut service = service_with_rows(test_props(), &[]);

    drop(service.send(Event::SortColumn {
        column: "name".to_string(),
    }));

    let attrs = service.connect(&|_| {}).column_header_attrs("name", true);

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::Sort))
            .map(ToString::to_string),
        Some("ascending".to_string()),
    );
    assert_snapshot!(
        "table_column_header_sortable_ascending",
        snapshot_attrs(&attrs)
    );
}

#[test]
fn table_column_header_sortable_aria_sort_descending_after_second_click() {
    let mut service = service_with_rows(test_props(), &[]);

    drop(service.send(Event::SortColumn {
        column: "name".to_string(),
    }));
    drop(service.send(Event::SortColumn {
        column: "name".to_string(),
    }));

    let attrs = service.connect(&|_| {}).column_header_attrs("name", true);

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::Sort))
            .map(ToString::to_string),
        Some("descending".to_string()),
    );
}

#[test]
fn table_column_header_sortable_aria_sort_none_after_third_click() {
    let mut service = service_with_rows(test_props(), &[]);

    drop(service.send(Event::SortColumn {
        column: "name".to_string(),
    }));
    drop(service.send(Event::SortColumn {
        column: "name".to_string(),
    }));
    drop(service.send(Event::SortColumn {
        column: "name".to_string(),
    }));

    let attrs = service.connect(&|_| {}).column_header_attrs("name", true);

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::Sort))
            .map(ToString::to_string),
        Some("none".to_string()),
    );
    assert!(service.context().sort_descriptor.get().is_none());
}

#[test]
fn table_column_header_non_sortable_omits_aria_sort() {
    let service = service_with_rows(test_props(), &[]);

    let attrs = service
        .connect(&|_| {})
        .column_header_attrs("avatar", false);

    assert!(attrs.get(&HtmlAttr::Aria(AriaAttr::Sort)).is_none());
    assert_snapshot!("table_column_header_non_sortable", snapshot_attrs(&attrs));
}

#[test]
fn table_sort_column_does_not_touch_is_sorting() {
    // `ctx.is_sorting` is adapter-controlled (Codex review PR #651). The
    // agnostic core's SortColumn transition only updates `sort_descriptor`
    // — for async sort indicators, adapters dispatch `Event::SetIsSorting`
    // explicitly. Synchronous sorts never expose the flag because the
    // sort completes in the same render frame.
    let mut service = service_with_rows(test_props(), &[]);

    assert!(!service.context().is_sorting);

    drop(service.send(Event::SortColumn {
        column: "name".to_string(),
    }));

    assert!(
        !service.context().is_sorting,
        "SortColumn must not flip is_sorting; use SetIsSorting from the adapter"
    );
}

// ────────────────────────────────────────────────────────────────────
// 3. Row selection — checkbox attrs + selection events
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_row_checkbox_aria_checked_false_default() {
    let service = service_with_rows(test_props_multi(), &[key("r1"), key("r2")]);

    let attrs = service.connect(&|_| {}).row_checkbox_attrs(&key("r1"));

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::Checked))
            .map(ToString::to_string),
        Some("false".to_string()),
    );
    assert_snapshot!("table_row_checkbox_unselected", snapshot_attrs(&attrs));
}

#[test]
fn table_row_checkbox_aria_checked_true_after_toggle() {
    let mut service = service_with_rows(test_props_multi(), &[key("r1"), key("r2")]);

    drop(service.send(Event::ToggleRow(key("r1"))));

    let attrs = service.connect(&|_| {}).row_checkbox_attrs(&key("r1"));

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::Checked))
            .map(ToString::to_string),
        Some("true".to_string()),
    );
    assert_snapshot!("table_row_checkbox_selected", snapshot_attrs(&attrs));
}

#[test]
fn table_row_attrs_selection_mode_none_omits_aria_selected() {
    let service = service_with_rows(test_props(), &[key("r1")]);

    let attrs = service.connect(&|_| {}).row_attrs(&key("r1"));

    assert!(attrs.get(&HtmlAttr::Aria(AriaAttr::Selected)).is_none());
}

#[test]
fn table_row_attrs_disabled_row() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("r1"));

    let service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    let attrs = service.connect(&|_| {}).row_attrs(&key("r1"));

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::Disabled))
            .map(ToString::to_string),
        Some("true".to_string()),
    );
    assert_snapshot!("table_row_disabled", snapshot_attrs(&attrs));
}

#[test]
fn table_select_row_rejected_when_selection_mode_none() {
    let mut service = service_with_rows(test_props(), &[key("r1")]);

    let result = service.send(Event::SelectRow(key("r1")));

    assert!(!result.context_changed);
    assert!(matches!(
        service.context().selected_rows.get(),
        selection::Set::Empty
    ));
}

#[test]
fn table_select_row_rejected_when_disabled() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("r1"));

    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    let result = service.send(Event::SelectRow(key("r1")));

    assert!(!result.context_changed);
}

// ────────────────────────────────────────────────────────────────────
// 4. SelectAll — mixed / true / false + Mode::None short-circuit
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_select_all_checkbox_false_when_no_rows_selected() {
    let service = service_with_rows(test_props_multi(), &[key("r1"), key("r2")]);

    let ids = [key("r1"), key("r2")];

    let id_refs = ids.iter().collect::<Vec<_>>();

    let attrs = service.connect(&|_| {}).select_all_attrs(&id_refs);

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::Checked))
            .map(ToString::to_string),
        Some("false".to_string()),
    );
    assert_snapshot!("table_select_all_unchecked", snapshot_attrs(&attrs));
}

#[test]
fn table_select_all_checkbox_mixed_when_partial() {
    let mut service = service_with_rows(test_props_multi(), &[key("r1"), key("r2"), key("r3")]);

    drop(service.send(Event::ToggleRow(key("r1"))));

    let ids = [key("r1"), key("r2"), key("r3")];

    let id_refs = ids.iter().collect::<Vec<_>>();

    let attrs = service.connect(&|_| {}).select_all_attrs(&id_refs);

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::Checked))
            .map(ToString::to_string),
        Some("mixed".to_string()),
    );
    assert!(attrs.get(&HtmlAttr::Data("ars-indeterminate")).is_some());
    assert_snapshot!("table_select_all_mixed", snapshot_attrs(&attrs));
}

#[test]
fn table_select_all_checkbox_true_when_all() {
    let mut service = service_with_rows(test_props_multi(), &[key("r1"), key("r2")]);
    drop(service.send(Event::SelectAll));

    let ids = [key("r1"), key("r2")];
    let id_refs = ids.iter().collect::<Vec<_>>();

    let attrs = service.connect(&|_| {}).select_all_attrs(&id_refs);

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::Checked))
            .map(ToString::to_string),
        Some("true".to_string()),
    );
    assert_snapshot!("table_select_all_checked", snapshot_attrs(&attrs));
}

#[test]
fn table_select_all_event_only_when_multiple() {
    let mut service = service_with_rows(test_props_single(), &[key("r1"), key("r2")]);

    let result = service.send(Event::SelectAll);

    assert!(!result.context_changed);
    assert!(matches!(
        service.context().selected_rows.get(),
        selection::Set::Empty
    ));
}

#[test]
fn table_select_all_mode_none_renders_empty_attrs() {
    let service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            select_all_mode: SelectAllMode::None,
            ..test_props()
        },
        &[key("r1")],
    );

    let attrs = service.connect(&|_| {}).select_all_attrs(&[]);

    assert!(attrs.iter_attrs().next().is_none());
}

#[test]
fn table_select_all_mode_all_data_label_includes_total_count() {
    let service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            select_all_mode: SelectAllMode::AllData { total_count: 1_204 },
            ..test_props()
        },
        &[key("r1")],
    );

    let attrs = service.connect(&|_| {}).select_all_attrs(&[&key("r1")]);

    let label = attrs
        .get(&HtmlAttr::Aria(AriaAttr::Label))
        .map(ToString::to_string)
        .unwrap_or_default();

    assert!(
        label.contains("1204"),
        "expected total count in label, got {label}"
    );
}

// ────────────────────────────────────────────────────────────────────
// 5. Expandable rows — aria-expanded + hidden detail row
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_expand_trigger_aria_expanded_false_default() {
    let service = service_with_rows(test_props(), &[key("r1")]);

    let attrs = service.connect(&|_| {}).expand_trigger_attrs(&key("r1"));

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::Expanded))
            .map(ToString::to_string),
        Some("false".to_string()),
    );
}

#[test]
fn table_expand_trigger_aria_expanded_after_event() {
    let mut service = service_with_rows(test_props(), &[key("r1")]);

    drop(service.send(Event::ExpandRow(key("r1"))));

    let api = service.connect(&|_| {});

    let trigger = api.expand_trigger_attrs(&key("r1"));

    assert_eq!(
        trigger
            .get(&HtmlAttr::Aria(AriaAttr::Expanded))
            .map(ToString::to_string),
        Some("true".to_string()),
    );
    assert!(trigger.get(&HtmlAttr::Data("ars-expanded")).is_some());

    let row = api.row_attrs(&key("r1"));

    assert_eq!(
        row.get(&HtmlAttr::Aria(AriaAttr::Expanded))
            .map(ToString::to_string),
        Some("true".to_string()),
    );

    assert_snapshot!("table_expand_trigger_expanded", snapshot_attrs(&trigger));
}

#[test]
fn table_expanded_content_hidden_when_collapsed() {
    let service = service_with_rows(test_props(), &[key("r1")]);

    let attrs = service.connect(&|_| {}).expanded_content_attrs(&key("r1"));

    assert!(attrs.get(&HtmlAttr::Hidden).is_some());
    assert_snapshot!("table_expanded_content_collapsed", snapshot_attrs(&attrs));
}

#[test]
fn table_expanded_content_visible_when_expanded() {
    let mut service = service_with_rows(test_props(), &[key("r1")]);

    drop(service.send(Event::ExpandRow(key("r1"))));

    let attrs = service.connect(&|_| {}).expanded_content_attrs(&key("r1"));

    assert!(attrs.get(&HtmlAttr::Hidden).is_none());
    assert_snapshot!("table_expanded_content_expanded", snapshot_attrs(&attrs));
}

#[test]
fn table_expand_row_rejected_when_disabled() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("r1"));

    let mut service = service_with_rows(
        Props {
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("r1")],
    );

    let result = service.send(Event::ExpandRow(key("r1")));

    assert!(!result.context_changed);
}

// ────────────────────────────────────────────────────────────────────
// 6. Column resize handle (§6)
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_column_resize_handle_role_separator() {
    let service = service_with_rows(test_props(), &[]);

    let attrs = service
        .connect(&|_| {})
        .column_resize_handle_attrs("name", 120.0);

    assert_eq!(
        attrs.get(&HtmlAttr::Role).map(ToString::to_string),
        Some("separator".to_string()),
    );
    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::Orientation))
            .map(ToString::to_string),
        Some("vertical".to_string()),
    );
    assert_snapshot!("table_column_resize_handle_idle", snapshot_attrs(&attrs));
}

#[test]
fn table_column_resize_handle_aria_valuenow_after_resize() {
    let mut service = service_with_rows(test_props(), &[]);

    drop(service.send(Event::ColumnResize {
        column: "name".to_string(),
        width: 220.0,
    }));

    let attrs = service
        .connect(&|_| {})
        .column_resize_handle_attrs("name", 120.0);

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::ValueNow))
            .map(ToString::to_string),
        Some("220".to_string()),
    );
    assert!(attrs.get(&HtmlAttr::Data("ars-resizing")).is_some());
    assert_snapshot!(
        "table_column_resize_handle_resizing",
        snapshot_attrs(&attrs)
    );
}

#[test]
fn table_column_resize_clamps_to_min_width() {
    let mut service = service_with_rows(test_props(), &[]);

    drop(service.send(Event::ColumnResize {
        column: "name".to_string(),
        width: 10.0,
    }));

    assert_eq!(service.context().column_widths.get("name"), Some(&50.0));
}

#[test]
fn table_column_resize_creates_entry_on_first_resize() {
    let mut service = service_with_rows(test_props(), &[]);

    assert!(service.context().column_widths.is_empty());

    drop(service.send(Event::ColumnResize {
        column: "name".to_string(),
        width: 180.0,
    }));

    assert_eq!(service.context().column_widths.get("name"), Some(&180.0));
}

#[test]
fn table_column_resize_end_clears_resizing_column() {
    let mut service = service_with_rows(test_props(), &[]);

    drop(service.send(Event::ColumnResize {
        column: "name".to_string(),
        width: 220.0,
    }));

    assert_eq!(service.context().resizing_column.as_deref(), Some("name"));

    drop(service.send(Event::ColumnResizeEnd {
        column: "name".to_string(),
    }));

    assert!(service.context().resizing_column.is_none());
}

#[test]
fn table_resize_handle_keydown_arrow_right_increases_width() {
    let service = service_with_rows(test_props(), &[]);

    let recorder = EventRecorder::default();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_resize_handle_keydown("name", 100.0, &keydown(KeyboardKey::ArrowRight));
    }

    let events = recorder.borrow();

    assert_eq!(events.len(), 1);

    match &events[0] {
        Event::ColumnResize { column, width } => {
            assert_eq!(column, "name");
            assert!(
                (*width - 110.0).abs() < f64::EPSILON,
                "expected 110.0, got {width}"
            );
        }

        other => panic!("expected ColumnResize, got {other:?}"),
    }
}

#[test]
fn table_resize_handle_keydown_arrow_left_decreases_width() {
    let service = service_with_rows(test_props(), &[]);

    let recorder = EventRecorder::default();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_resize_handle_keydown("name", 100.0, &keydown(KeyboardKey::ArrowLeft));
    }

    let events = recorder.borrow();

    match &events[0] {
        Event::ColumnResize { width, .. } => {
            assert!(
                (*width - 90.0).abs() < f64::EPSILON,
                "expected 90.0, got {width}"
            );
        }

        other => panic!("expected ColumnResize, got {other:?}"),
    }
}

#[test]
fn table_resize_handle_keydown_rtl_flips() {
    let service = service_with_rows(
        Props {
            dir: Direction::Rtl,
            ..test_props()
        },
        &[],
    );

    let recorder = EventRecorder::default();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_resize_handle_keydown("name", 100.0, &keydown(KeyboardKey::ArrowRight));
    }

    let events = recorder.borrow();

    match &events[0] {
        Event::ColumnResize { width, .. } => {
            assert!(
                (*width - 90.0).abs() < f64::EPSILON,
                "RTL ArrowRight should shrink — expected 90.0, got {width}",
            );
        }

        other => panic!("expected ColumnResize, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────
// 7. Keyboard navigation across cells
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_on_cell_keydown_arrow_right_emits_focus_cell_next() {
    let service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1"), key("r2")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_cell_keydown(
            &key("r1"),
            1,
            &keydown(KeyboardKey::ArrowRight),
            &row_refs,
            4,
        );
    }

    let events = recorder.borrow();

    assert_eq!(events.len(), 1);

    match &events[0] {
        Event::FocusCell {
            row,
            col,
            row_index,
        } => {
            assert_eq!(row, &key("r1"));
            assert_eq!(*col, 2);
            assert_eq!(*row_index, 0);
        }

        other => panic!("expected FocusCell, got {other:?}"),
    }
}

#[test]
fn table_on_cell_keydown_arrow_right_rtl_emits_prev() {
    let service = service_with_rows(
        Props {
            interactive: true,
            dir: Direction::Rtl,
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1"), key("r2")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_cell_keydown(
            &key("r1"),
            2,
            &keydown(KeyboardKey::ArrowRight),
            &row_refs,
            4,
        );
    }

    let events = recorder.borrow();

    match &events[0] {
        Event::FocusCell { col, .. } => {
            assert_eq!(*col, 1, "RTL ArrowRight should decrement column");
        }

        other => panic!("expected FocusCell, got {other:?}"),
    }
}

#[test]
fn table_on_cell_keydown_home_jumps_to_col_zero() {
    let service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_cell_keydown(&key("r1"), 3, &keydown(KeyboardKey::Home), &row_refs, 5);
    }

    match &recorder.borrow()[0] {
        Event::FocusCell { col, row_index, .. } => {
            assert_eq!(*col, 0);
            assert_eq!(*row_index, 0);
        }

        other => panic!("expected FocusCell, got {other:?}"),
    }
}

#[test]
fn table_ctrl_home_jumps_to_first_cell_of_first_row() {
    let service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1"), key("r2"), key("r3")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_cell_keydown(
            &key("r3"),
            4,
            &ctrl_keydown(KeyboardKey::Home),
            &row_refs,
            5,
        );
    }

    match &recorder.borrow()[0] {
        Event::FocusCell {
            row,
            col,
            row_index,
        } => {
            assert_eq!(row, &key("r1"));
            assert_eq!(*col, 0);
            assert_eq!(*row_index, 0);
        }

        other => panic!("expected FocusCell, got {other:?}"),
    }
}

#[test]
fn table_ctrl_end_jumps_to_last_cell_of_last_row() {
    let service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1"), key("r2"), key("r3")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_cell_keydown(&key("r1"), 0, &ctrl_keydown(KeyboardKey::End), &row_refs, 5);
    }

    match &recorder.borrow()[0] {
        Event::FocusCell {
            row,
            col,
            row_index,
        } => {
            assert_eq!(row, &key("r3"));
            assert_eq!(*col, 4);
            assert_eq!(*row_index, 2);
        }

        other => panic!("expected FocusCell, got {other:?}"),
    }
}

#[test]
fn table_cell_attrs_roving_tabindex_when_interactive() {
    let mut service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1")],
    );

    drop(service.send(Event::Focus { cell: (2, 0) }));

    let api = service.connect(&|_| {});

    let focused = api.cell_attrs(2, 0);

    assert_eq!(
        focused.get(&HtmlAttr::TabIndex).map(ToString::to_string),
        Some("0".to_string()),
    );

    let unfocused = api.cell_attrs(1, 0);

    assert_eq!(
        unfocused.get(&HtmlAttr::TabIndex).map(ToString::to_string),
        Some("-1".to_string()),
    );
}

#[test]
fn table_blur_clears_all_three_focus_fields() {
    let mut service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1")],
    );

    drop(service.send(Event::FocusCell {
        row: key("r1"),
        col: 2,
        row_index: 0,
    }));

    assert!(service.context().focused_cell.is_some());
    assert!(service.context().focused_row.is_some());
    assert!(service.context().focused_col.is_some());

    drop(service.send(Event::Blur));

    assert!(service.context().focused_cell.is_none());
    assert!(service.context().focused_row.is_none());
    assert!(service.context().focused_col.is_none());
}

// ────────────────────────────────────────────────────────────────────
// 8. Empty state
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_empty_state_renders_zero_rows() {
    let service = service_with_rows(test_props_multi(), &[]);

    let api = service.connect(&|_| {});

    let select_all = api.select_all_attrs(&[]);

    assert_eq!(
        select_all
            .get(&HtmlAttr::Aria(AriaAttr::Checked))
            .map(ToString::to_string),
        Some("false".to_string()),
    );
    assert!(
        select_all
            .get(&HtmlAttr::Data("ars-indeterminate"))
            .is_none()
    );

    assert!(api.context().rows.is_empty());
}

// ────────────────────────────────────────────────────────────────────
// 9. Row actions
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_row_action_passes_through_for_enabled_row() {
    let mut service = service_with_rows(test_props(), &[key("r1")]);

    // RowAction is a pure notification — the transition plan is
    // `context_only(|_| {})` so the contract is "the event is accepted
    // and the no-op closure runs without panicking", not that context
    // changed.
    drop(service.send(Event::RowAction(key("r1"))));
}

#[test]
fn table_row_action_event_disabled_row_no_op() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("r1"));

    let mut service = service_with_rows(
        Props {
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("r1")],
    );

    let result = service.send(Event::RowAction(key("r1")));

    assert!(!result.state_changed);
    assert!(!result.context_changed);
}

// ────────────────────────────────────────────────────────────────────
// 10. Virtual scrolling — aria-rowcount / aria-colcount / aria-rowindex
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_aria_rowcount_aria_colcount_virtualized() {
    let service = service_with_rows(
        Props {
            virtual_scrolling: true,
            total_rows: 1_500,
            total_cols: 8,
            ..test_props()
        },
        &[],
    );

    let attrs = service.connect(&|_| {}).table_attrs();

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::RowCount))
            .map(ToString::to_string),
        Some("1500".to_string()),
    );
    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::ColCount))
            .map(ToString::to_string),
        Some("8".to_string()),
    );
    assert_snapshot!("table_table_virtualized", snapshot_attrs(&attrs));
}

#[test]
fn table_aria_rowindex_on_row_when_virtualized() {
    let service = service_with_rows(
        Props {
            virtual_scrolling: true,
            total_rows: 1_500,
            total_cols: 4,
            ..test_props()
        },
        &[],
    );

    let attrs = service.connect(&|_| {}).row_attrs_indexed(&key("r42"), 41);

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::RowIndex))
            .map(ToString::to_string),
        Some("42".to_string()),
    );
}

#[test]
fn table_aria_rowcount_aria_colcount_omitted_when_not_virtualized() {
    let service = service_with_rows(test_props(), &[]);

    let attrs = service.connect(&|_| {}).table_attrs();

    assert!(attrs.get(&HtmlAttr::Aria(AriaAttr::RowCount)).is_none());
    assert!(attrs.get(&HtmlAttr::Aria(AriaAttr::ColCount)).is_none());
}

#[test]
fn table_set_row_counts_updates_context() {
    let mut service = service_with_rows(test_props(), &[]);

    drop(service.send(Event::SetRowCounts {
        total_rows: 500,
        total_cols: 6,
    }));

    assert_eq!(service.context().total_rows, 500);
    assert_eq!(service.context().total_cols, 6);
}

// ────────────────────────────────────────────────────────────────────
// 11. disallow_empty_selection
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_disallow_empty_selection_blocks_last_deselect() {
    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            disallow_empty_selection: true,
            default_selected_rows: selection::Set::Single(key("r1")),
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    let result = service.send(Event::DeselectRow(key("r1")));

    assert!(!result.context_changed);
    assert!(service.context().selected_rows.get().contains(&key("r1")));
}

#[test]
fn table_disallow_empty_selection_blocks_deselect_all() {
    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            disallow_empty_selection: true,
            default_selected_rows: selection::Set::Single(key("r1")),
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    let result = service.send(Event::DeselectAll);

    assert!(!result.context_changed);
    assert!(!service.context().selected_rows.get().is_empty());
}

#[test]
fn table_disallow_empty_selection_blocks_toggle_of_last() {
    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            disallow_empty_selection: true,
            default_selected_rows: selection::Set::Single(key("r1")),
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    let result = service.send(Event::ToggleRow(key("r1")));

    assert!(!result.context_changed);
    assert!(service.context().selected_rows.get().contains(&key("r1")));
}

// ────────────────────────────────────────────────────────────────────
// 12. Escape key
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_escape_clears_selection_when_clear_selection_mode() {
    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            default_selected_rows: selection::Set::Single(key("r1")),
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    assert!(!service.context().selected_rows.get().is_empty());

    drop(service.send(Event::EscapeKey));

    assert!(service.context().selected_rows.get().is_empty());
}

#[test]
fn table_escape_no_op_when_behavior_none() {
    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            escape_key_behavior: EscapeKeyBehavior::None,
            default_selected_rows: selection::Set::Single(key("r1")),
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    let result = service.send(Event::EscapeKey);

    assert!(!result.context_changed);
    assert!(!service.context().selected_rows.get().is_empty());
}

#[test]
fn table_escape_blocked_by_disallow_empty_selection() {
    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            disallow_empty_selection: true,
            default_selected_rows: selection::Set::Single(key("r1")),
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    let result = service.send(Event::EscapeKey);

    assert!(!result.context_changed);
}

// ────────────────────────────────────────────────────────────────────
// 13. Init pruning + SetRows
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_init_prunes_disabled_default_selection() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("r1"));

    let service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            disabled_keys: disabled,
            default_selected_rows: selection::Set::Single(key("r1")),
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    assert!(service.context().selected_rows.get().is_empty(),);
}

#[test]
fn table_set_rows_prunes_stale_selection_and_focus() {
    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            interactive: true,
            default_selected_rows: selection::Set::Single(key("r2")),
            default_expanded_rows: {
                let mut s = BTreeSet::new();
                s.insert(key("r2"));
                s
            },
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );

    drop(service.send(Event::FocusRow(key("r2"))));

    assert!(service.context().selected_rows.get().contains(&key("r2")));
    assert!(service.context().expanded_rows.get().contains(&key("r2")));
    assert_eq!(service.context().focused_row.as_ref(), Some(&key("r2")));

    drop(service.send(Event::SetRows(vec![key("r1"), key("r3")])));

    assert!(!service.context().selected_rows.get().contains(&key("r2")));
    assert!(!service.context().expanded_rows.get().contains(&key("r2")));
    assert!(service.context().focused_row.is_none());
}

#[test]
fn table_set_direction_idempotent() {
    let mut service = service_with_rows(test_props(), &[]);

    let initial_dir = service.context().dir;

    let result = service.send(Event::SetDirection(initial_dir));

    assert!(!result.context_changed);
}

#[test]
fn table_set_direction_changes_dir() {
    let mut service = service_with_rows(test_props(), &[]);

    drop(service.send(Event::SetDirection(Direction::Rtl)));

    assert_eq!(service.context().dir, Direction::Rtl);
}

// ────────────────────────────────────────────────────────────────────
// 14. Sync controlled values
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_sync_controlled_selected_rows() {
    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            selected_rows: Some(selection::Set::Empty),
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    assert!(service.context().selected_rows.get().is_empty());

    drop(service.send(Event::SyncControlledSelectedRows(Some(
        selection::Set::Single(key("r1")),
    ))));

    assert!(service.context().selected_rows.get().contains(&key("r1")));
    assert!(
        service
            .context()
            .selection_state
            .selected_keys
            .contains(&key("r1"))
    );
}

#[test]
fn table_set_loading_round_trips_via_bindable() {
    let mut service = service_with_rows(test_props(), &[]);

    assert!(!*service.context().loading.get());

    drop(service.send(Event::SetLoading(true)));

    assert!(*service.context().loading.get());

    drop(service.send(Event::SetLoading(false)));

    assert!(!*service.context().loading.get());
}

// ────────────────────────────────────────────────────────────────────
// 15. Custom messages — verifies MessageFn plumbing
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_custom_select_row_message_is_used() {
    let messages = Messages {
        select_row: MessageFn::new(|_locale: &Locale| "Auswählen".to_string()),
        ..Messages::default()
    };

    let mut service = Service::<Machine>::new(test_props_multi(), &Env::default(), &messages);

    drop(service.send(Event::SetRows(vec![key("r1")])));

    let attrs = service.connect(&|_| {}).row_checkbox_attrs(&key("r1"));

    let label = attrs
        .get(&HtmlAttr::Aria(AriaAttr::Label))
        .map(ToString::to_string)
        .unwrap_or_default();

    assert_eq!(label, "Auswählen");
}

// ────────────────────────────────────────────────────────────────────
// 16. Additional coverage — positive-case selection / expansion /
//      row-link attrs / RowAction transition acceptance / SetRowCounts
//      idempotency
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_select_row_accepted_for_enabled_row() {
    let mut service = service_with_rows(test_props_multi(), &[key("r1"), key("r2")]);

    let result = service.send(Event::SelectRow(key("r1")));

    assert!(result.context_changed);
    assert!(service.context().selected_rows.get().contains(&key("r1")));
    assert!(service.context().selection_state.is_selected(&key("r1")));
}

#[test]
fn table_select_row_in_single_mode_replaces() {
    let mut service = service_with_rows(test_props_single(), &[key("r1"), key("r2")]);

    drop(service.send(Event::SelectRow(key("r1"))));
    drop(service.send(Event::SelectRow(key("r2"))));

    assert!(service.context().selected_rows.get().contains(&key("r2")));
    // `selection::State::select` in `Mode::Single` replaces — verify that
    // the canonical `selected_keys` no longer carries the first key.
    assert!(!service.context().selection_state.is_selected(&key("r1")));
}

#[test]
fn table_row_link_attrs_adds_data_ars_href() {
    let service = service_with_rows(test_props(), &[key("r1")]);

    let attrs = service
        .connect(&|_| {})
        .row_link_attrs(&key("r1"), "/users/r1");

    assert_eq!(
        attrs
            .get(&HtmlAttr::Data("ars-href"))
            .map(ToString::to_string),
        Some("/users/r1".to_string()),
    );
}

#[test]
fn table_expand_row_event_inserts_into_expanded_set() {
    let mut service = service_with_rows(test_props(), &[key("r1")]);

    let result = service.send(Event::ExpandRow(key("r1")));

    assert!(result.context_changed);
    assert!(service.context().expanded_rows.get().contains(&key("r1")));
}

#[test]
fn table_expand_row_idempotent_when_already_expanded() {
    let mut service = service_with_rows(test_props(), &[key("r1")]);

    drop(service.send(Event::ExpandRow(key("r1"))));

    let result = service.send(Event::ExpandRow(key("r1")));

    assert!(!result.context_changed);
}

#[test]
fn table_collapse_row_removes_from_expanded_set() {
    let mut service = service_with_rows(test_props(), &[key("r1")]);

    drop(service.send(Event::ExpandRow(key("r1"))));

    assert!(service.context().expanded_rows.get().contains(&key("r1")));

    let result = service.send(Event::CollapseRow(key("r1")));

    assert!(result.context_changed);
    assert!(!service.context().expanded_rows.get().contains(&key("r1")));
}

#[test]
fn table_collapse_row_idempotent_when_not_expanded() {
    let mut service = service_with_rows(test_props(), &[key("r1")]);

    let result = service.send(Event::CollapseRow(key("r1")));

    assert!(!result.context_changed);
}

#[test]
fn table_row_action_transition_accepted_for_enabled_row() {
    let mut service = service_with_rows(test_props(), &[key("r1")]);

    let result = service.send(Event::RowAction(key("r1")));

    // RowAction is a pure notification, so context doesn't change, but
    // the transition MUST be accepted (not None) so the adapter's
    // `on_row_action` callback fires. `state_changed` is false because
    // the transition has no target state.
    assert!(!result.state_changed);
}

#[test]
fn table_set_row_counts_idempotent_when_unchanged() {
    let mut service = service_with_rows(
        Props {
            total_rows: 5,
            total_cols: 3,
            ..test_props()
        },
        &[],
    );

    let result = service.send(Event::SetRowCounts {
        total_rows: 5,
        total_cols: 3,
    });

    assert!(!result.context_changed);
}

#[test]
fn table_init_seeds_selection_state_from_default_selected_rows() {
    // Regression guard for spec drift D1: `selection_state.selected_keys`
    // must reflect `default_selected_rows` at init so subsequent
    // `SelectRow` / `ToggleRow` guards observe the correct boot state.
    let service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            default_selected_rows: selection::Set::Single(key("r1")),
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    assert!(service.context().selection_state.is_selected(&key("r1")));
    assert!(!service.context().selection_state.is_selected(&key("r2")));
}

// ────────────────────────────────────────────────────────────────────
// 17. Coverage backfill — mutation-survivor closure
// ────────────────────────────────────────────────────────────────────

#[test]
fn table_sort_column_resets_cycle_when_switching_columns() {
    // Regression guard for the `desc.column == *column` match-guard
    // mutation. Sorting column A ascending, then sort column B, must
    // reset B to Ascending (not flip).
    let mut service = service_with_rows(test_props(), &[]);

    drop(service.send(Event::SortColumn {
        column: "name".to_string(),
    }));
    drop(service.send(Event::SortColumn {
        column: "name".to_string(),
    }));
    // `name` is now Descending. Switching to `date` should be Ascending.
    drop(service.send(Event::SortColumn {
        column: "date".to_string(),
    }));

    let descriptor = service.context().sort_descriptor.get().clone();

    assert_eq!(descriptor.as_ref().map(|d| d.column.as_str()), Some("date"),);
    assert_eq!(
        descriptor.as_ref().map(|d| d.direction),
        Some(SortDirection::Ascending),
    );
}

#[test]
fn table_set_rows_prunes_multi_selection() {
    // Exercises `restrict_selection_to_rows` on the `Set::Multiple` arm
    // (mutation-survivor coverage). Starts with three rows multi-selected,
    // then `SetRows` drops one row — selection must lose that row only.
    let mut multi = BTreeSet::new();

    multi.insert(key("r1"));
    multi.insert(key("r2"));
    multi.insert(key("r3"));

    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            default_selected_rows: selection::Set::Multiple(multi),
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );

    drop(service.send(Event::SetRows(vec![key("r1"), key("r3")])));

    let sel = service.context().selected_rows.get().clone();

    assert!(sel.contains(&key("r1")));
    assert!(!sel.contains(&key("r2")));
    assert!(sel.contains(&key("r3")));
}

#[test]
fn table_init_prunes_multiple_disabled_selection() {
    // Exercises `prune_selection_against` on the `Set::Multiple` arm —
    // the boot selection contains two disabled keys and one enabled.
    // The enabled one must survive as `Single`.
    let mut disabled = BTreeSet::new();

    disabled.insert(key("r1"));
    disabled.insert(key("r3"));

    let mut initial = BTreeSet::new();

    initial.insert(key("r1"));
    initial.insert(key("r2"));
    initial.insert(key("r3"));

    let service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            disabled_keys: disabled,
            default_selected_rows: selection::Set::Multiple(initial),
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );

    let sel = service.context().selected_rows.get().clone();

    assert!(!sel.contains(&key("r1")));
    assert!(sel.contains(&key("r2")));
    assert!(!sel.contains(&key("r3")));
}

#[test]
fn table_on_row_keydown_arrow_down_emits_focus_row() {
    let service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1"), key("r2")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_row_keydown(&key("r1"), &keydown(KeyboardKey::ArrowDown), &row_refs);
    }

    match &recorder.borrow()[0] {
        Event::FocusRow(k) => assert_eq!(k, &key("r2")),
        other => panic!("expected FocusRow, got {other:?}"),
    }
}

#[test]
fn table_on_row_keydown_arrow_up_emits_focus_row() {
    let service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1"), key("r2")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_row_keydown(&key("r2"), &keydown(KeyboardKey::ArrowUp), &row_refs);
    }

    match &recorder.borrow()[0] {
        Event::FocusRow(k) => assert_eq!(k, &key("r1")),
        other => panic!("expected FocusRow, got {other:?}"),
    }
}

#[test]
fn table_on_row_keydown_enter_emits_toggle_row() {
    let service = service_with_rows(
        Props {
            interactive: true,
            selection_mode: selection::Mode::Multiple,
            ..test_props()
        },
        &[key("r1")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_row_keydown(&key("r1"), &keydown(KeyboardKey::Enter), &row_refs);
    }

    assert!(matches!(recorder.borrow()[0], Event::ToggleRow(_)));
}

#[test]
fn table_on_row_keydown_escape_emits_escape_key() {
    let service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_row_keydown(&key("r1"), &keydown(KeyboardKey::Escape), &row_refs);
    }

    assert!(matches!(recorder.borrow()[0], Event::EscapeKey));
}

#[test]
fn table_focus_event_rejected_when_non_interactive() {
    let mut service = service_with_rows(test_props(), &[key("r1")]);

    let result = service.send(Event::Focus { cell: (1, 0) });

    assert!(!result.context_changed);
}

#[test]
fn table_focus_event_accepted_when_interactive() {
    let mut service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1")],
    );

    let result = service.send(Event::Focus { cell: (2, 1) });

    assert!(result.context_changed);
    assert_eq!(service.context().focused_cell, Some((2, 1)));
}

#[test]
fn table_sync_controlled_expanded_rows() {
    let mut service = service_with_rows(
        Props {
            expanded_rows: Some(BTreeSet::new()),
            ..test_props()
        },
        &[key("r1")],
    );

    let mut new_expansion = BTreeSet::new();

    new_expansion.insert(key("r1"));

    drop(service.send(Event::SyncControlledExpandedRows(Some(new_expansion))));

    assert!(service.context().expanded_rows.get().contains(&key("r1")));
}

#[test]
fn table_sync_controlled_sort_descriptor_pushes_value() {
    // `SyncControlledSortDescriptor` mirrors a parent-driven controlled
    // value into the Bindable. `is_sorting` is no longer coupled to the
    // sort transition — adapters drive it explicitly via `SetIsSorting`.
    let mut service = service_with_rows(
        Props {
            sort_descriptor: Bindable::controlled(None),
            ..test_props()
        },
        &[],
    );
    assert!(service.context().sort_descriptor.get().is_none());

    // Outer `Some(...)` = enter / update controlled mode; inner `Some(...)` is the
    // sort value (a `SortDescriptor`). `Some(None)` would mean "controlled,
    // no active sort"; `None` would exit controlled mode entirely.
    drop(service.send(Event::SyncControlledSortDescriptor(Some(Some(
        SortDescriptor {
            column: "name".to_string(),
            direction: SortDirection::Ascending,
        },
    )))));

    let descriptor = service.context().sort_descriptor.get().clone();
    assert_eq!(descriptor.as_ref().map(|d| d.column.as_str()), Some("name"),);
}

#[test]
fn table_on_row_keydown_home_emits_focus_row_first() {
    let service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1"), key("r2"), key("r3")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_row_keydown(&key("r3"), &keydown(KeyboardKey::Home), &row_refs);
    }

    match &recorder.borrow()[0] {
        Event::FocusRow(k) => assert_eq!(k, &key("r1")),
        other => panic!("expected FocusRow, got {other:?}"),
    }
}

#[test]
fn table_on_row_keydown_end_emits_focus_row_last() {
    let service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1"), key("r2"), key("r3")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_row_keydown(&key("r1"), &keydown(KeyboardKey::End), &row_refs);
    }

    match &recorder.borrow()[0] {
        Event::FocusRow(k) => assert_eq!(k, &key("r3")),
        other => panic!("expected FocusRow, got {other:?}"),
    }
}

#[test]
fn table_on_row_keydown_space_emits_toggle_row() {
    let service = service_with_rows(
        Props {
            interactive: true,
            selection_mode: selection::Mode::Multiple,
            ..test_props()
        },
        &[key("r1")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_row_keydown(&key("r1"), &keydown(KeyboardKey::Space), &row_refs);
    }

    assert!(matches!(recorder.borrow()[0], Event::ToggleRow(_)));
}

#[test]
fn table_on_row_keydown_unknown_key_no_op() {
    let service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_row_keydown(&key("r1"), &keydown(KeyboardKey::Tab), &row_refs);
    }

    assert!(recorder.borrow().is_empty());
}

#[test]
fn table_on_cell_keydown_arrow_down() {
    let service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1"), key("r2")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_cell_keydown(
            &key("r1"),
            0,
            &keydown(KeyboardKey::ArrowDown),
            &row_refs,
            3,
        );
    }

    match &recorder.borrow()[0] {
        Event::FocusCell {
            row,
            col,
            row_index,
        } => {
            assert_eq!(row, &key("r2"));
            assert_eq!(*col, 0);
            assert_eq!(*row_index, 1);
        }

        other => panic!("expected FocusCell, got {other:?}"),
    }
}

#[test]
fn table_on_cell_keydown_arrow_up() {
    let service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1"), key("r2")];

    let row_refs = rows.iter().collect::<Vec<_>>();
    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_cell_keydown(&key("r2"), 0, &keydown(KeyboardKey::ArrowUp), &row_refs, 3);
    }

    match &recorder.borrow()[0] {
        Event::FocusCell { row, row_index, .. } => {
            assert_eq!(row, &key("r1"));
            assert_eq!(*row_index, 0);
        }

        other => panic!("expected FocusCell, got {other:?}"),
    }
}

#[test]
fn table_on_cell_keydown_end_jumps_to_last_col() {
    let service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_cell_keydown(&key("r1"), 1, &keydown(KeyboardKey::End), &row_refs, 5);
    }

    match &recorder.borrow()[0] {
        Event::FocusCell { col, .. } => assert_eq!(*col, 4),
        other => panic!("expected FocusCell, got {other:?}"),
    }
}

#[test]
fn table_on_cell_keydown_space_emits_toggle_row() {
    let service = service_with_rows(
        Props {
            interactive: true,
            selection_mode: selection::Mode::Multiple,
            ..test_props()
        },
        &[key("r1")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_cell_keydown(&key("r1"), 0, &keydown(KeyboardKey::Space), &row_refs, 3);
    }

    assert!(matches!(recorder.borrow()[0], Event::ToggleRow(_)));
}

#[test]
fn table_on_cell_keydown_escape_emits_escape_key() {
    let service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1")],
    );

    let recorder = EventRecorder::default();

    let rows = [key("r1")];

    let row_refs = rows.iter().collect::<Vec<_>>();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_cell_keydown(&key("r1"), 0, &keydown(KeyboardKey::Escape), &row_refs, 3);
    }

    assert!(matches!(recorder.borrow()[0], Event::EscapeKey));
}

#[test]
fn table_on_resize_handle_keydown_unknown_key_no_op() {
    let service = service_with_rows(test_props(), &[]);

    let recorder = EventRecorder::default();

    {
        let send = |e| record(&recorder, e);

        let api = service.connect(&send);

        api.on_resize_handle_keydown("name", 100.0, &keydown(KeyboardKey::Enter));
    }

    assert!(recorder.borrow().is_empty());
}

#[test]
fn table_part_attrs_dispatcher_covers_every_variant() {
    // Drive every `Part` variant through `ConnectApi::part_attrs` so
    // the dispatcher arms remain covered as the enum grows.
    let mut service = service_with_rows(test_props_multi(), &[key("r1")]);

    drop(service.send(Event::ExpandRow(key("r1"))));

    let api = service.connect(&|_| {});

    let parts = [
        Part::Root,
        Part::Table,
        Part::Caption,
        Part::Head,
        Part::Body,
        Part::Foot,
        Part::Row { key: key("r1") },
        Part::ColumnHeader {
            header: "name".to_string(),
            sortable: true,
        },
        Part::RowHeader,
        Part::Cell { col: 0, row: 0 },
        Part::SelectAllCheckbox,
        Part::RowCheckbox { key: key("r1") },
        Part::ExpandTrigger { key: key("r1") },
        Part::ExpandedContent { key: key("r1") },
        Part::ColumnResizeHandle {
            column: "name".to_string(),
        },
    ];

    for part in parts {
        let attrs = api.part_attrs(part);
        // Every part should yield at least the scope/part attrs unless
        // intentionally short-circuited (SelectAllCheckbox in mode
        // None — but we're in Multiple here, so all are populated).
        assert!(
            attrs.iter_attrs().next().is_some(),
            "expected non-empty AttrMap for the dispatched part"
        );
    }
}

// ────────────────────────────────────────────────────────────────────
// 18. Codex review (PR #651) — regression guards for findings that
//      surfaced in the first automated review pass.
// ────────────────────────────────────────────────────────────────────

#[test]
fn codex_sync_props_copies_updated_disabled_keys_into_context() {
    // Codex P1 (thread PRRT_kwDORp4enM6DRmcb).
    // SyncProps must mirror Props → Context for the non-Bindable fields.
    let service = service_with_rows(test_props_multi(), &[key("r1"), key("r2")]);
    assert!(service.context().disabled_keys.is_empty());

    let mut new_disabled = BTreeSet::new();
    new_disabled.insert(key("r1"));
    let new_props = Props {
        selection_mode: selection::Mode::Multiple,
        disabled_keys: new_disabled,
        ..test_props_multi()
    };

    let mut service = service;
    drop(service.set_props(new_props));

    assert!(
        service.context().disabled_keys.contains(&key("r1")),
        "SyncProps must copy disabled_keys from new props into Context"
    );
}

#[test]
fn codex_sync_props_copies_updated_interactive_flag() {
    // Codex P1 — SyncProps interactivity update.
    let service = service_with_rows(test_props(), &[key("r1")]);
    assert!(!service.context().interactive);

    let new_props = Props {
        interactive: true,
        ..test_props()
    };
    let mut service = service;
    drop(service.set_props(new_props));

    assert!(service.context().interactive);
}

// Note: the original pass-1 tests
// `codex_toggle_row_materializes_set_all_for_individual_deselect` and
// `codex_deselect_row_materializes_set_all_for_individual_deselect`
// were superseded by the round-4 contracts:
//
//   * `AllData` mode preserves `Set::All` — see
//     `codex_round4_toggle_row_keeps_set_all_in_all_data_mode` and
//     `codex_round4_deselect_row_keeps_set_all_in_all_data_mode`.
//   * `AllVisible` mode never writes `Set::All` from `Event::SelectAll`
//     — it materializes to `Multiple(ctx.rows)` at the SelectAll step
//     so there's nothing left to materialize on deselect.
//
// The `materialize_all_against_rows` helper is retained as a defensive
// net for the edge case where `Set::All` enters selection via a
// controlled `selected_rows` Bindable that subsequently exits
// controlled mode — that path is exercised by the round-3 tests on
// `SyncControlledSelectedRows`.

#[test]
fn codex_on_props_changed_dispatches_sync_events() {
    // Codex P1 (thread PRRT_kwDORp4enM6DRmck).
    // `Machine::on_props_changed` must emit the appropriate sync events
    // when props differ between renders. We verify via `Service::set_props`
    // which calls `on_props_changed` and then dispatches the returned
    // events.
    let old_props = test_props_multi();
    let mut disabled = BTreeSet::new();
    disabled.insert(key("r1"));
    let new_props = Props {
        disabled_keys: disabled.clone(),
        dir: Direction::Rtl,
        ..test_props_multi()
    };

    let service = service_with_rows(old_props, &[key("r1"), key("r2")]);
    let mut service = service;
    drop(service.set_props(new_props));

    // `SetDirection` must have flowed.
    assert_eq!(service.context().dir, Direction::Rtl);
    // `SyncProps` must have copied the new disabled_keys.
    assert_eq!(service.context().disabled_keys, disabled);
}

#[test]
fn codex_select_all_in_all_visible_mode_materializes_current_rows() {
    // Codex P1 (thread PRRT_kwDORp4enM6DRmc3).
    // Default `SelectAllMode::AllVisible` must NOT write `Set::All` —
    // that variant means "every row in the dataset including unloaded
    // ones". Visible-only select-all materializes the registered row set.
    let mut service = service_with_rows(test_props_multi(), &[key("r1"), key("r2")]);

    drop(service.send(Event::SelectAll));

    let sel = service.context().selected_rows.get().clone();
    assert!(
        !matches!(sel, selection::Set::All),
        "AllVisible select-all must not write Set::All — that means dataset-wide"
    );
    assert!(sel.contains(&key("r1")));
    assert!(sel.contains(&key("r2")));
}

#[test]
fn codex_select_all_in_all_data_mode_writes_set_all() {
    // Codex P1 (companion to the previous test). `AllData` mode keeps
    // the `Set::All` semantics — adapter knows the total row count and
    // wants the global-select-with-exclusions pattern.
    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            select_all_mode: SelectAllMode::AllData { total_count: 1_204 },
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    drop(service.send(Event::SelectAll));

    assert!(matches!(
        service.context().selected_rows.get(),
        selection::Set::All
    ));
}

#[test]
fn codex_sort_column_does_not_leave_is_sorting_stuck() {
    // Codex P2 (thread PRRT_kwDORp4enM6DRmcs).
    // The agnostic `SortColumn` transition must not leave `is_sorting`
    // stuck `true` in the synchronous-sort path. The flag becomes
    // adapter-controlled via `SetIsSorting(bool)`.
    let mut service = service_with_rows(test_props(), &[]);
    drop(service.send(Event::SortColumn {
        column: "name".to_string(),
    }));

    assert!(
        !service.context().is_sorting,
        "SortColumn must not set is_sorting in the synchronous path; adapters opt-in via SetIsSorting"
    );
}

#[test]
fn codex_set_is_sorting_event_toggles_flag() {
    // Codex P2 follow-up — the new `SetIsSorting` event lets adapters
    // drive the async-sort indicator explicitly.
    let mut service = service_with_rows(test_props(), &[]);
    assert!(!service.context().is_sorting);

    drop(service.send(Event::SetIsSorting(true)));
    assert!(service.context().is_sorting);

    drop(service.send(Event::SetIsSorting(false)));
    assert!(!service.context().is_sorting);
}

#[test]
fn codex_select_all_attrs_gated_on_mode_multiple() {
    // Codex P2 (thread PRRT_kwDORp4enM6DRmcy).
    // `select_all_attrs` must return an empty AttrMap in Single / None
    // selection modes — even when `SelectAllMode::AllVisible` is set —
    // because `Event::SelectAll` is rejected in those modes and the
    // checkbox would render but never work.
    let service = service_with_rows(test_props_single(), &[key("r1")]);
    let attrs = service.connect(&|_| {}).select_all_attrs(&[&key("r1")]);

    assert!(
        attrs.iter_attrs().next().is_none(),
        "select_all_attrs must short-circuit when selection_mode != Multiple, got {attrs:?}"
    );
}

#[test]
fn codex_select_all_attrs_empty_when_mode_none() {
    let service = service_with_rows(test_props(), &[key("r1")]);
    let attrs = service.connect(&|_| {}).select_all_attrs(&[&key("r1")]);

    assert!(attrs.iter_attrs().next().is_none());
}

#[test]
fn codex_set_rows_rebases_focused_cell_when_row_moves_index() {
    // Codex P2 (thread PRRT_kwDORp4enM6DRmdB).
    // When the focused row stays in the new row list but moves to a new
    // index (sort/filter/reorder), `focused_cell.1` must update to track
    // the new position so adapter focus wiring targets the right cell.
    let mut service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );
    drop(service.send(Event::FocusCell {
        row: key("r2"),
        col: 1,
        row_index: 1,
    }));
    assert_eq!(service.context().focused_cell, Some((1, 1)));

    // Reorder: r2 was at index 1, now at index 0.
    drop(service.send(Event::SetRows(vec![key("r2"), key("r3"), key("r1")])));

    assert_eq!(
        service.context().focused_cell,
        Some((1, 0)),
        "SetRows must rebase focused_cell row-index to the new position"
    );
    assert_eq!(service.context().focused_row.as_ref(), Some(&key("r2")));
}

// ────────────────────────────────────────────────────────────────────
// 19. Codex review pass 2 (PR #651) — regression guards for findings
//      that surfaced after the first round of Codex fixes landed.
// ────────────────────────────────────────────────────────────────────

#[test]
fn codex_round2_uncontrolled_to_controlled_sort_dispatches_sync() {
    // Codex P1 (thread PRRT_kwDORp4enM6DSWzU).
    // Switching `Props.sort_descriptor` from uncontrolled to controlled
    // between renders must propagate the new controlled value into
    // `Context::sort_descriptor`. Without this fix the Bindable stayed
    // uncontrolled and parent-controlled sort silently drifted.
    let old_props = Props {
        sort_descriptor: Bindable::uncontrolled(None),
        ..test_props()
    };
    let new_props = Props {
        sort_descriptor: Bindable::controlled(Some(SortDescriptor {
            column: "name".to_string(),
            direction: SortDirection::Ascending,
        })),
        ..test_props()
    };

    let service = service_with_rows(old_props, &[]);
    let mut service = service;
    drop(service.set_props(new_props));

    assert!(
        service.context().sort_descriptor.is_controlled(),
        "Bindable must switch to controlled mode after props change"
    );
    assert_eq!(
        service
            .context()
            .sort_descriptor
            .get()
            .as_ref()
            .map(|d| d.column.as_str()),
        Some("name"),
    );
}

#[test]
fn codex_round2_controlled_to_uncontrolled_sort_dispatches_sync() {
    // Codex P1 (thread PRRT_kwDORp4enM6DSWzd).
    // The dual case: parent moves the prop back to uncontrolled. The
    // Bindable must exit controlled mode, which requires the sync event
    // to carry the outer "leave controlled mode" toggle as `None`.
    let old_props = Props {
        sort_descriptor: Bindable::controlled(Some(SortDescriptor {
            column: "name".to_string(),
            direction: SortDirection::Ascending,
        })),
        ..test_props()
    };
    let new_props = Props {
        sort_descriptor: Bindable::uncontrolled(None),
        ..test_props()
    };

    let service = service_with_rows(old_props, &[]);
    let mut service = service;
    drop(service.set_props(new_props));

    assert!(
        !service.context().sort_descriptor.is_controlled(),
        "Bindable must exit controlled mode after props change to uncontrolled"
    );
}

#[test]
fn codex_round2_controlled_loading_syncs_on_props_change() {
    // Codex P1 (thread PRRT_kwDORp4enM6DSWzm).
    // `on_props_changed` was omitting `loading`, so controlled-loading
    // updates from the parent never reached `Context::loading`.
    let old_props = Props {
        loading: Bindable::controlled(false),
        ..test_props()
    };
    let new_props = Props {
        loading: Bindable::controlled(true),
        ..test_props()
    };

    let service = service_with_rows(old_props, &[]);
    let mut service = service;
    drop(service.set_props(new_props));

    assert!(
        *service.context().loading.get(),
        "controlled loading prop must flow into Context"
    );
}

#[test]
fn codex_round2_sync_props_does_not_corrupt_controlled_selection() {
    // Codex P1 (thread PRRT_kwDORp4enM6DSWzs).
    // For a controlled `selected_rows`, `set(...)` updates the internal
    // value but `get()` keeps returning the external controlled value.
    // SyncProps must not silently desync `selection_state.selected_keys`
    // from the actually-visible selection — re-sync from `get()`.
    let mut disabled = BTreeSet::new();
    disabled.insert(key("r1"));

    let mut controlled = BTreeSet::new();
    controlled.insert(key("r1"));
    controlled.insert(key("r2"));

    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            selected_rows: Some(selection::Set::Multiple(controlled.clone())),
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    // Parent renders with new disabled_keys.
    let new_props = Props {
        selection_mode: selection::Mode::Multiple,
        selected_rows: Some(selection::Set::Multiple(controlled)),
        disabled_keys: disabled.clone(),
        ..test_props()
    };
    drop(service.set_props(new_props));

    // For controlled bindables, the user-visible value is whatever the
    // parent passed — including any disabled keys. The contract is
    // "parent owns the value; pruning is best-effort". What MUST hold
    // is that `selection_state.selected_keys` matches what the API
    // surface reports (`selected_rows.get()`), not a divergent internal
    // value.
    assert_eq!(
        &service.context().selection_state.selected_keys,
        service.context().selected_rows.get(),
        "selection_state.selected_keys must track the actual current value",
    );
}

#[test]
fn codex_round2_sync_props_prunes_uncontrolled_selection() {
    // Codex P1 follow-up — pruning still applies in the uncontrolled
    // case where the agnostic core owns the value.
    let mut new_disabled = BTreeSet::new();
    new_disabled.insert(key("r1"));

    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            default_selected_rows: selection::Set::Multiple({
                let mut s = BTreeSet::new();
                s.insert(key("r1"));
                s.insert(key("r2"));
                s
            }),
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );

    let new_props = Props {
        selection_mode: selection::Mode::Multiple,
        disabled_keys: new_disabled,
        default_selected_rows: selection::Set::Multiple({
            let mut s = BTreeSet::new();
            s.insert(key("r1"));
            s.insert(key("r2"));
            s
        }),
        ..test_props()
    };
    drop(service.set_props(new_props));

    let sel = service.context().selected_rows.get().clone();
    assert!(
        !sel.contains(&key("r1")),
        "uncontrolled selection must be pruned of disabled rows"
    );
    assert!(sel.contains(&key("r2")));
}

#[test]
fn codex_round2_sync_props_resyncs_id_and_caption_id() {
    // Codex P2 (thread PRRT_kwDORp4enM6DSWzg).
    // `Props.id` change must propagate to `Context.id` and
    // `Context.caption_id`, otherwise ARIA wiring breaks (the
    // `aria-labelledby` and `aria-controls` ids stay stale after a
    // parent rename).
    let mut service = service_with_rows(test_props(), &[]);
    assert_eq!(service.context().id, "table");
    assert_eq!(service.context().caption_id, "table-caption");

    let new_props = Props {
        id: "orders".to_string(),
        ..test_props()
    };
    drop(service.set_props(new_props));

    assert_eq!(
        service.context().id,
        "orders",
        "Context.id must follow Props.id"
    );
    assert_eq!(
        service.context().caption_id,
        "orders-caption",
        "caption_id must be derived from the new base id",
    );
}

#[test]
fn codex_round2_part_attrs_select_all_uses_ctx_rows() {
    // Codex P2 (thread PRRT_kwDORp4enM6DSWzi).
    // `ConnectApi::part_attrs(Part::SelectAllCheckbox)` previously
    // called `select_all_attrs(&[])`, always reporting an empty row set
    // and therefore "aria-checked=false" even when every registered row
    // was selected. The dispatcher must route through `ctx.rows`.
    let mut service = service_with_rows(test_props_multi(), &[key("r1"), key("r2")]);
    drop(service.send(Event::SelectAll));

    let attrs = service.connect(&|_| {}).part_attrs(Part::SelectAllCheckbox);

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::Checked))
            .map(ToString::to_string),
        Some("true".to_string()),
        "part_attrs dispatcher must drive aria-checked from ctx.rows, not an empty slice",
    );
}

// ────────────────────────────────────────────────────────────────────
// 20. Codex review pass 3 (PR #651) — selection_state alignment with
//      the user-visible `selected_rows.get()` value across SetRows and
//      SyncControlledSelectedRows transitions.
// ────────────────────────────────────────────────────────────────────

#[test]
fn codex_round3_set_rows_keeps_selection_state_aligned_when_controlled() {
    // Codex P1 (thread PRRT_kwDORp4enM6DSv8K).
    // For controlled `selected_rows`, `set(...)` only updates the
    // internal fallback. `selection_state.selected_keys` must follow
    // what `get()` actually returns (the parent's value), not the
    // pruned/restricted internal write — otherwise transition guards
    // and API reads disagree.
    let mut controlled = BTreeSet::new();
    controlled.insert(key("r1"));
    controlled.insert(key("r2"));
    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            selected_rows: Some(selection::Set::Multiple(controlled)),
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );

    // Drop r2 from the registered rows. `restrict_selection_to_rows`
    // would normally prune r2 out of the *internal* fallback, but the
    // controlled value still carries r2 — selection_state must mirror
    // what `selected_rows.get()` reports.
    drop(service.send(Event::SetRows(vec![key("r1"), key("r3")])));

    assert_eq!(
        &service.context().selection_state.selected_keys,
        service.context().selected_rows.get(),
        "selection_state must track `selected_rows.get()` regardless of controlled mode",
    );
}

#[test]
fn codex_round3_sync_controlled_selected_rows_leave_controlled_resyncs_state() {
    // Codex P2 (thread PRRT_kwDORp4enM6DSv8M).
    // When `SyncControlledSelectedRows(None)` switches the Bindable
    // back to uncontrolled, the internal fallback (set during init)
    // may differ from the last controlled value. After the transition,
    // `selection_state.selected_keys` must reflect the internal value
    // that `get()` now returns.
    let mut controlled = BTreeSet::new();
    controlled.insert(key("r1"));
    controlled.insert(key("r2"));
    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            selected_rows: Some(selection::Set::Multiple(controlled.clone())),
            // Internal fallback (used when uncontrolled) is empty —
            // different from the controlled value so the desync would
            // be visible.
            default_selected_rows: selection::Set::Empty,
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );
    assert_eq!(
        service.context().selection_state.selected_keys,
        selection::Set::Multiple(controlled),
    );

    // Leave controlled mode.
    drop(service.send(Event::SyncControlledSelectedRows(None)));

    assert_eq!(
        &service.context().selection_state.selected_keys,
        service.context().selected_rows.get(),
        "selection_state must follow `selected_rows.get()` after leaving controlled mode",
    );
}

// ────────────────────────────────────────────────────────────────────
// 21. Codex review pass 4 (PR #651) — semantic + a11y findings around
//      AllData mode, disabled-row keyboard navigation, multiselect,
//      and row-link virtualization.
// ────────────────────────────────────────────────────────────────────

#[test]
fn codex_round4_deselect_row_keeps_set_all_in_all_data_mode() {
    // Codex P1 (thread PRRT_kwDORp4enM6DTElS).
    // For paginated `AllData` tables, the agnostic core MUST NOT
    // silently downgrade `Set::All` → `Multiple(rows)` on the first
    // individual deselect — that drops every unloaded row from
    // selection. The adapter tracks exclusions via §5.2
    // `BulkSelection`; the core leaves `Set::All` alone.
    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            select_all_mode: SelectAllMode::AllData {
                total_count: 10_000,
            },
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );
    drop(service.send(Event::SelectAll));
    assert!(matches!(
        service.context().selected_rows.get(),
        selection::Set::All
    ));

    drop(service.send(Event::DeselectRow(key("r2"))));

    assert!(
        matches!(service.context().selected_rows.get(), selection::Set::All),
        "AllData mode must keep `Set::All` after individual deselect — adapter tracks the exclusion via BulkSelection",
    );
}

#[test]
fn codex_round4_toggle_row_keeps_set_all_in_all_data_mode() {
    // Codex P1 follow-up — same contract via ToggleRow.
    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            select_all_mode: SelectAllMode::AllData {
                total_count: 10_000,
            },
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );
    drop(service.send(Event::SelectAll));

    drop(service.send(Event::ToggleRow(key("r2"))));

    assert!(
        matches!(service.context().selected_rows.get(), selection::Set::All),
        "AllData mode must keep `Set::All` after toggle on individual row",
    );
}

#[test]
fn codex_round4_on_row_keydown_skips_disabled_when_arrow_down() {
    // Codex P2 (thread PRRT_kwDORp4enM6DTElY).
    // Arrow-key row navigation must skip disabled rows so adapter focus
    // wiring doesn't land on `aria-disabled` rows.
    let mut disabled = BTreeSet::new();
    disabled.insert(key("r2"));
    let service = service_with_rows(
        Props {
            interactive: true,
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );
    let recorder = EventRecorder::default();
    let rows = [key("r1"), key("r2"), key("r3")];
    let row_refs: Vec<&Key> = rows.iter().collect();
    {
        let send = |e| record(&recorder, e);
        let api = service.connect(&send);
        api.on_row_keydown(&key("r1"), &keydown(KeyboardKey::ArrowDown), &row_refs);
    }

    match &recorder.borrow()[0] {
        Event::FocusRow(k) => assert_eq!(k, &key("r3"), "must skip disabled r2"),
        other => panic!("expected FocusRow(r3), got {other:?}"),
    }
}

#[test]
fn codex_round4_on_row_keydown_skips_disabled_when_arrow_up() {
    let mut disabled = BTreeSet::new();
    disabled.insert(key("r2"));
    let service = service_with_rows(
        Props {
            interactive: true,
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );
    let recorder = EventRecorder::default();
    let rows = [key("r1"), key("r2"), key("r3")];
    let row_refs: Vec<&Key> = rows.iter().collect();
    {
        let send = |e| record(&recorder, e);
        let api = service.connect(&send);
        api.on_row_keydown(&key("r3"), &keydown(KeyboardKey::ArrowUp), &row_refs);
    }

    match &recorder.borrow()[0] {
        Event::FocusRow(k) => assert_eq!(k, &key("r1"), "must skip disabled r2"),
        other => panic!("expected FocusRow(r1), got {other:?}"),
    }
}

#[test]
fn codex_round4_on_cell_keydown_arrow_down_skips_disabled_row() {
    // Codex P2 (thread PRRT_kwDORp4enM6DTElb).
    let mut disabled = BTreeSet::new();
    disabled.insert(key("r2"));
    let service = service_with_rows(
        Props {
            interactive: true,
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );
    let recorder = EventRecorder::default();
    let rows = [key("r1"), key("r2"), key("r3")];
    let row_refs: Vec<&Key> = rows.iter().collect();
    {
        let send = |e| record(&recorder, e);
        let api = service.connect(&send);
        api.on_cell_keydown(
            &key("r1"),
            0,
            &keydown(KeyboardKey::ArrowDown),
            &row_refs,
            3,
        );
    }

    match &recorder.borrow()[0] {
        Event::FocusCell { row, row_index, .. } => {
            assert_eq!(row, &key("r3"), "must skip disabled r2");
            assert_eq!(*row_index, 2);
        }
        other => panic!("expected FocusCell(r3), got {other:?}"),
    }
}

#[test]
fn codex_round4_table_attrs_emits_aria_multiselectable_on_interactive_grid() {
    // Codex P2 (thread PRRT_kwDORp4enM6DTEld).
    let service = service_with_rows(
        Props {
            interactive: true,
            selection_mode: selection::Mode::Multiple,
            ..test_props()
        },
        &[key("r1")],
    );
    let attrs = service.connect(&|_| {}).table_attrs();

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::MultiSelectable))
            .map(ToString::to_string),
        Some("true".to_string()),
        "interactive grid with Mode::Multiple must advertise aria-multiselectable",
    );
}

#[test]
fn codex_round4_table_attrs_omits_aria_multiselectable_in_single_mode() {
    let service = service_with_rows(
        Props {
            interactive: true,
            selection_mode: selection::Mode::Single,
            ..test_props()
        },
        &[key("r1")],
    );
    let attrs = service.connect(&|_| {}).table_attrs();

    assert!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::MultiSelectable))
            .is_none(),
        "Mode::Single must NOT emit aria-multiselectable",
    );
}

#[test]
fn codex_round4_table_attrs_omits_aria_multiselectable_when_non_interactive() {
    let service = service_with_rows(
        Props {
            interactive: false,
            selection_mode: selection::Mode::Multiple,
            ..test_props()
        },
        &[key("r1")],
    );
    let attrs = service.connect(&|_| {}).table_attrs();

    assert!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::MultiSelectable))
            .is_none(),
        "non-interactive `role=table` must NOT emit aria-multiselectable",
    );
}

#[test]
fn codex_round4_row_link_attrs_indexed_preserves_aria_rowindex() {
    // Codex P2 (thread PRRT_kwDORp4enM6DTElh).
    // `row_link_attrs` currently delegates to `row_attrs` which hardcodes
    // row_index=0, breaking aria-rowindex on virtualized tables. The
    // indexed variant must propagate the row position.
    let service = service_with_rows(
        Props {
            virtual_scrolling: true,
            total_rows: 1_000,
            total_cols: 4,
            ..test_props()
        },
        &[],
    );

    let attrs = service
        .connect(&|_| {})
        .row_link_attrs_indexed(&key("r99"), "/orders/99", 98);

    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::RowIndex))
            .map(ToString::to_string),
        Some("99".to_string()),
        "row_link_attrs_indexed must propagate the row index into aria-rowindex",
    );
    assert_eq!(
        attrs
            .get(&HtmlAttr::Data("ars-href"))
            .map(ToString::to_string),
        Some("/orders/99".to_string()),
    );
}

#[test]
fn codex_round4_on_row_keydown_home_skips_disabled_first_row() {
    // Home must land on the first ENABLED row, skipping any leading
    // disabled rows.
    let mut disabled = BTreeSet::new();
    disabled.insert(key("r1"));
    let service = service_with_rows(
        Props {
            interactive: true,
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );
    let recorder = EventRecorder::default();
    let rows = [key("r1"), key("r2"), key("r3")];
    let row_refs: Vec<&Key> = rows.iter().collect();
    {
        let send = |e| record(&recorder, e);
        let api = service.connect(&send);
        api.on_row_keydown(&key("r3"), &keydown(KeyboardKey::Home), &row_refs);
    }

    match &recorder.borrow()[0] {
        Event::FocusRow(k) => assert_eq!(k, &key("r2"), "Home must skip disabled r1"),
        other => panic!("expected FocusRow(r2), got {other:?}"),
    }
}

#[test]
fn codex_round4_on_row_keydown_end_skips_disabled_last_row() {
    let mut disabled = BTreeSet::new();
    disabled.insert(key("r3"));
    let service = service_with_rows(
        Props {
            interactive: true,
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );
    let recorder = EventRecorder::default();
    let rows = [key("r1"), key("r2"), key("r3")];
    let row_refs: Vec<&Key> = rows.iter().collect();
    {
        let send = |e| record(&recorder, e);
        let api = service.connect(&send);
        api.on_row_keydown(&key("r1"), &keydown(KeyboardKey::End), &row_refs);
    }

    match &recorder.borrow()[0] {
        Event::FocusRow(k) => assert_eq!(k, &key("r2"), "End must skip disabled r3"),
        other => panic!("expected FocusRow(r2), got {other:?}"),
    }
}

#[test]
fn codex_round4_on_row_keydown_arrow_no_op_when_all_remaining_disabled() {
    // When every row after the current one is disabled, ArrowDown
    // must emit no event (no FocusRow to land on).
    let mut disabled = BTreeSet::new();
    disabled.insert(key("r2"));
    disabled.insert(key("r3"));
    let service = service_with_rows(
        Props {
            interactive: true,
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );
    let recorder = EventRecorder::default();
    let rows = [key("r1"), key("r2"), key("r3")];
    let row_refs: Vec<&Key> = rows.iter().collect();
    {
        let send = |e| record(&recorder, e);
        let api = service.connect(&send);
        api.on_row_keydown(&key("r1"), &keydown(KeyboardKey::ArrowDown), &row_refs);
    }

    assert!(
        recorder.borrow().is_empty(),
        "no event when no enabled successor"
    );
}

#[test]
fn codex_round4_ctrl_home_skips_disabled_first_row_in_cell_nav() {
    let mut disabled = BTreeSet::new();
    disabled.insert(key("r1"));
    let service = service_with_rows(
        Props {
            interactive: true,
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );
    let recorder = EventRecorder::default();
    let rows = [key("r1"), key("r2"), key("r3")];
    let row_refs: Vec<&Key> = rows.iter().collect();
    {
        let send = |e| record(&recorder, e);
        let api = service.connect(&send);
        api.on_cell_keydown(
            &key("r3"),
            2,
            &ctrl_keydown(KeyboardKey::Home),
            &row_refs,
            4,
        );
    }

    match &recorder.borrow()[0] {
        Event::FocusCell { row, row_index, .. } => {
            assert_eq!(row, &key("r2"), "Ctrl+Home must skip disabled r1");
            assert_eq!(*row_index, 1);
        }
        other => panic!("expected FocusCell(r2), got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────
// 22. Codex review pass 5 (PR #651) — width clamping, disabled-row
//      semantics on SelectAll, focused_cell rebase without focused_row,
//      and Part::ColumnHeader sortable dispatch.
// ────────────────────────────────────────────────────────────────────

#[test]
fn codex_round5_sync_props_reclamps_column_widths_below_new_min() {
    // Codex P2 (thread PRRT_kwDORp4enM6DTd50).
    // After `Props::min_column_width` increases, existing cached widths
    // that fall below the new minimum must be re-clamped — otherwise
    // adapter rendering surfaces values that violate the active
    // constraint until the user resizes again.
    let mut service = service_with_rows(
        Props {
            min_column_width: 50.0,
            ..test_props()
        },
        &[],
    );
    drop(service.send(Event::ColumnResize {
        column: "name".to_string(),
        width: 60.0,
    }));
    assert_eq!(service.context().column_widths.get("name"), Some(&60.0));

    let new_props = Props {
        min_column_width: 100.0,
        ..test_props()
    };
    drop(service.set_props(new_props));

    assert_eq!(
        service.context().column_widths.get("name"),
        Some(&100.0),
        "existing column_widths must be re-clamped against the new minimum",
    );
}

#[test]
fn codex_round5_is_row_selected_excludes_disabled_in_set_all() {
    // Codex P2 (thread PRRT_kwDORp4enM6DTd56).
    // `is_row_selected` must return `false` for disabled rows even when
    // selection is `Set::All` — the contract is "disabled rows are
    // non-selectable", not "disabled rows happen to be selected when
    // everyone is".
    let mut disabled = BTreeSet::new();
    disabled.insert(key("r1"));
    let service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            select_all_mode: SelectAllMode::AllData { total_count: 3 },
            selected_rows: Some(selection::Set::All),
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("r1"), key("r2")],
    );
    let api = service.connect(&|_| {});

    assert!(
        !api.is_row_selected(&key("r1")),
        "disabled row must not report as selected under Set::All",
    );
    assert!(api.is_row_selected(&key("r2")));
}

#[test]
fn codex_round5_all_selected_excludes_disabled() {
    // Codex P2 (thread PRRT_kwDORp4enM6DTd5_).
    // When the user has selected every selectable row in a multi-select
    // table that contains disabled rows, the header checkbox should
    // report `aria-checked="true"`, not `"mixed"`. `all_selected` must
    // filter the disabled set so the result reflects "every SELECTABLE
    // row is selected".
    let mut disabled = BTreeSet::new();
    disabled.insert(key("r3"));
    let mut service = service_with_rows(
        Props {
            selection_mode: selection::Mode::Multiple,
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );
    drop(service.send(Event::SelectAll));

    let ids = [key("r1"), key("r2"), key("r3")];
    let id_refs: Vec<&Key> = ids.iter().collect();
    let api = service.connect(&|_| {});

    assert!(
        api.all_selected(&id_refs),
        "all_selected must filter disabled rows when checking",
    );
    let attrs = api.select_all_attrs(&id_refs);
    assert_eq!(
        attrs
            .get(&HtmlAttr::Aria(AriaAttr::Checked))
            .map(ToString::to_string),
        Some("true".to_string()),
    );
}

#[test]
fn codex_round5_set_rows_rebases_focused_cell_without_focused_row() {
    // Codex P2 (thread PRRT_kwDORp4enM6DTd6B).
    // `Event::Focus { cell }` populates `focused_cell` but NOT
    // `focused_row`. After SetRows, the row index inside focused_cell
    // can fall out of range. Clear it instead of leaving a stale
    // pointer that breaks the roving tabindex.
    let mut service = service_with_rows(
        Props {
            interactive: true,
            ..test_props()
        },
        &[key("r1"), key("r2"), key("r3")],
    );
    drop(service.send(Event::Focus { cell: (1, 2) }));
    assert_eq!(service.context().focused_cell, Some((1, 2)));
    assert!(service.context().focused_row.is_none());

    // Shrink the row list — index 2 is no longer valid.
    drop(service.send(Event::SetRows(vec![key("r1")])));

    assert!(
        service.context().focused_cell.is_none(),
        "focused_cell row index must be cleared when out of bounds after SetRows",
    );
}

#[test]
fn codex_round5_part_attrs_column_header_sortable_field() {
    // Codex P2 (thread PRRT_kwDORp4enM6DTd6C).
    // `Part::ColumnHeader` now carries a `sortable: bool` so the
    // dispatcher honors caller intent.
    let service = service_with_rows(test_props(), &[]);
    let api = service.connect(&|_| {});

    let sortable = api.part_attrs(Part::ColumnHeader {
        header: "name".to_string(),
        sortable: true,
    });
    assert!(
        sortable.get(&HtmlAttr::Aria(AriaAttr::Sort)).is_some(),
        "sortable=true must emit aria-sort",
    );

    let non_sortable = api.part_attrs(Part::ColumnHeader {
        header: "avatar".to_string(),
        sortable: false,
    });
    assert!(
        non_sortable.get(&HtmlAttr::Aria(AriaAttr::Sort)).is_none(),
        "sortable=false must omit aria-sort",
    );
}

// ────────────────────────────────────────────────────────────────────
// 23. Codex review pass 6 (PR #651) — Direction::Auto resolution in
//      SetDirection transition.
// ────────────────────────────────────────────────────────────────────

#[test]
fn codex_round6_set_direction_resolves_auto_from_locale() {
    // Codex P2 (thread PRRT_kwDORp4enM6DT49S).
    // `SetDirection(Direction::Auto)` must resolve `Auto` to a concrete
    // direction via the active locale rather than storing `Auto` in
    // `Context::dir`. Keyboard handlers compare `ctx.dir == Direction::Rtl`
    // and treat everything else (including `Auto`) as LTR, so leaving
    // `Auto` in context silently breaks RTL navigation after prop
    // updates.
    let mut service = service_with_rows(
        Props {
            dir: Direction::Rtl,
            ..test_props()
        },
        &[],
    );
    assert_eq!(service.context().dir, Direction::Rtl);

    // Adapter / parent flips the prop back to `Auto` (asks the
    // platform to re-resolve). The default locale is LTR, so the
    // resolved value should be `Ltr` — NOT `Auto`.
    drop(service.send(Event::SetDirection(Direction::Auto)));

    assert_eq!(
        service.context().dir,
        Direction::Ltr,
        "SetDirection(Auto) must resolve to a concrete direction via ctx.locale",
    );
}

#[test]
fn codex_round6_set_direction_auto_via_props_change() {
    // Companion test — `on_props_changed` forwards `new.dir` through
    // `Event::SetDirection`. A `Rtl → Auto` prop transition must end
    // with a concrete value in `ctx.dir`, not `Auto`.
    let mut service = service_with_rows(
        Props {
            dir: Direction::Rtl,
            ..test_props()
        },
        &[],
    );
    assert_eq!(service.context().dir, Direction::Rtl);

    let new_props = Props {
        dir: Direction::Auto,
        ..test_props()
    };
    drop(service.set_props(new_props));

    assert_ne!(
        service.context().dir,
        Direction::Auto,
        "Context::dir must always be a concrete Direction (Ltr or Rtl), never Auto",
    );
}
