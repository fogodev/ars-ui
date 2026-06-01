//! Unit and snapshot tests for the `RangeCalendar` component.

use alloc::{format, string::String, sync::Arc};

use ars_core::{AriaAttr, AttrMap, Callback, ComponentPart, Env, HtmlAttr, KeyboardKey, Service};
use ars_i18n::{
    CalendarDate, DateRange, Locale, StubIntlBackend, Weekday,
    locales::{en_gb, en_us, fa},
};
use insta::assert_snapshot;

use super::*;

fn date(year: i32, month: u8, day: u8) -> CalendarDate {
    CalendarDate::new_gregorian(year, month, day).expect("valid test date")
}

fn range(start: CalendarDate, end: CalendarDate) -> DateRange {
    DateRange::new(start, end).expect("valid test range")
}

fn props() -> Props {
    Props::new().id("range-cal").today(date(2024, 1, 15))
}

fn env(locale: Locale) -> Env {
    Env::new(locale, Arc::new(StubIntlBackend))
}

fn service() -> Service<Machine> {
    Service::<Machine>::new(props(), &env(en_us()), &Messages::default())
}

fn service_with(props: Props, locale: Locale) -> Service<Machine> {
    Service::<Machine>::new(props, &env(locale), &Messages::default())
}

fn snapshot_attrs(attrs: &AttrMap) -> String {
    format!("{attrs:#?}")
}

fn attr(attrs: &AttrMap, key: HtmlAttr) -> Option<String> {
    attrs.get(&key).map(ToString::to_string)
}

#[test]
fn first_click_sets_anchor_and_second_click_completes_range() {
    let mut svc = service();

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 10),
    }));

    assert_eq!(svc.context().anchor_date, Some(date(2024, 1, 10)));
    assert_eq!(*svc.context().value.get(), None);

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 15),
    }));

    assert_eq!(
        *svc.context().value.get(),
        Some(range(date(2024, 1, 10), date(2024, 1, 15))),
    );
    assert_eq!(svc.context().anchor_date, None);
    assert_eq!(svc.context().hovering_date, None);
}

#[test]
fn second_click_normalizes_reverse_order_range() {
    let mut svc = service();

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 20),
    }));
    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 12),
    }));

    assert_eq!(
        *svc.context().value.get(),
        Some(range(date(2024, 1, 12), date(2024, 1, 20))),
    );
}

#[test]
fn hover_preview_sets_and_clears_hover_range_attributes() {
    let mut svc = service();

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 10),
    }));
    drop(svc.send(Event::HoverDate {
        date: date(2024, 1, 13),
    }));

    let api = svc.connect(&|_| {});

    let trigger = api.cell_trigger_attrs(&date(2024, 1, 12));

    assert_eq!(
        attr(&trigger, HtmlAttr::Data("ars-in-hover-range")).as_deref(),
        Some("true"),
    );

    drop(svc.send(Event::HoverEnd));

    let api = svc.connect(&|_| {});

    let trigger = api.cell_trigger_attrs(&date(2024, 1, 12));

    assert!(trigger.get(&HtmlAttr::Data("ars-in-hover-range")).is_none());
}

#[test]
fn cells_in_confirmed_range_have_aria_selected() {
    let svc = service_with(
        props().value(Some(range(date(2024, 1, 10), date(2024, 1, 12)))),
        en_us(),
    );

    let api = svc.connect(&|_| {});

    for day in 10..=12 {
        let attrs = api.cell_attrs(&date(2024, 1, day));

        assert_eq!(
            attr(&attrs, HtmlAttr::Aria(AriaAttr::Selected)).as_deref(),
            Some("true"),
        );
    }
}

#[test]
fn grid_attrs_mark_range_selection_as_multiselectable() {
    let svc = service_with(props().visible_months(2), en_us());

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.grid_attrs(), HtmlAttr::Role).as_deref(),
        Some("grid")
    );
    assert_eq!(
        attr(&api.grid_attrs(), HtmlAttr::Aria(AriaAttr::MultiSelectable),).as_deref(),
        Some("true"),
    );
    assert_eq!(
        attr(&api.grid_group_attrs(), HtmlAttr::Role).as_deref(),
        Some("group"),
    );
}

#[test]
fn keyboard_enter_and_space_follow_two_step_range_selection() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));
    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::Enter,
        shift: false,
    }));
    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::ArrowRight,
        shift: false,
    }));
    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::Space,
        shift: false,
    }));

    assert_eq!(
        *svc.context().value.get(),
        Some(range(date(2024, 1, 15), date(2024, 1, 16))),
    );
}

#[test]
fn shift_page_keys_navigate_years() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));
    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::PageDown,
        shift: true,
    }));

    assert_eq!(svc.context().visible_year, 2025);

    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::PageUp,
        shift: true,
    }));

    assert_eq!(svc.context().visible_year, 2024);
}

#[test]
fn min_max_and_unavailable_dates_block_selection() {
    let unavailable = Callback::new_ref(|date: &CalendarDate| date.day() == 12);

    let mut svc = service_with(
        props()
            .min(Some(date(2024, 1, 10)))
            .max(Some(date(2024, 1, 20)))
            .is_date_unavailable(Some(unavailable)),
        en_us(),
    );

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 5),
    }));

    assert_eq!(svc.context().anchor_date, None);

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 12),
    }));

    assert_eq!(svc.context().anchor_date, None);
}

#[test]
fn range_span_constraints_keep_anchor_pending_when_invalid() {
    let mut svc = service_with(props().max_range_days(Some(3)), en_us());

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 10),
    }));
    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 15),
    }));

    assert_eq!(*svc.context().value.get(), None);
    assert_eq!(svc.context().anchor_date, Some(date(2024, 1, 10)));

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 12),
    }));

    assert_eq!(
        *svc.context().value.get(),
        Some(range(date(2024, 1, 10), date(2024, 1, 12))),
    );
}

#[test]
fn min_range_days_and_single_date_option_are_enforced() {
    let mut svc = service_with(
        props()
            .allow_single_date_range(false)
            .min_range_days(Some(2)),
        en_us(),
    );

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 10),
    }));
    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 10),
    }));

    assert_eq!(*svc.context().value.get(), None);
    assert_eq!(svc.context().anchor_date, Some(date(2024, 1, 10)));

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 11),
    }));

    assert_eq!(
        *svc.context().value.get(),
        Some(range(date(2024, 1, 10), date(2024, 1, 11))),
    );
}

#[test]
fn controlled_value_sync_preserves_pending_anchor() {
    let mut svc = service_with(
        props().value(Some(range(date(2024, 1, 1), date(2024, 1, 2)))),
        en_us(),
    );

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 10),
    }));
    drop(svc.set_props(props().value(Some(range(date(2024, 1, 3), date(2024, 1, 4))))));

    assert_eq!(svc.context().anchor_date, Some(date(2024, 1, 10)));
    assert_eq!(
        *svc.context().value.get(),
        Some(range(date(2024, 1, 3), date(2024, 1, 4))),
    );
}

#[test]
fn rtl_arrow_keys_swap_horizontal_direction() {
    let mut svc = service_with(props().is_rtl(true), fa());

    drop(svc.send(Event::FocusIn));

    let before = svc.context().focused_date.clone();

    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::ArrowLeft,
        shift: false,
    }));

    assert_eq!(svc.context().focused_date, before.add_days(1).unwrap());
}

#[test]
fn week_day_labels_follow_locale() {
    let svc = service_with(props(), en_gb());

    assert_eq!(
        svc.context()
            .week_day_labels()
            .first()
            .map(|(weekday, _)| *weekday),
        Some(Weekday::Monday),
    );
}

#[test]
fn part_anatomy_matches_spec() {
    assert_eq!(Part::scope(), "range-calendar");

    let names: Vec<&'static str> = Part::all().iter().map(ComponentPart::name).collect();

    let expected: &[&'static str] = &[
        "root",
        "header",
        "prev-trigger",
        "next-trigger",
        "heading",
        "grid",
        "grid-group",
        "head-row",
        "head-cell",
        "row",
        "cell",
        "cell-trigger",
    ];

    assert_eq!(names.as_slice(), expected);
}

#[test]
fn snapshot_root_idle() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_idle", snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_pending() {
    let mut svc = service();

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 10),
    }));

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_pending", snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_disabled_readonly_rtl() {
    let svc = service_with(props().disabled(true).readonly(true).is_rtl(true), en_us());

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "root_disabled_readonly_rtl",
        snapshot_attrs(&api.root_attrs()),
    );
}

#[test]
fn snapshot_structural_parts() {
    let svc = service_with(props().visible_months(2), en_us());

    let api = svc.connect(&|_| {});

    assert_snapshot!("header", snapshot_attrs(&api.header_attrs()));
    assert_snapshot!("prev_trigger", snapshot_attrs(&api.prev_trigger_attrs()));
    assert_snapshot!("next_trigger", snapshot_attrs(&api.next_trigger_attrs()));
    assert_snapshot!("heading", snapshot_attrs(&api.heading_attrs()));
    assert_snapshot!("grid", snapshot_attrs(&api.grid_attrs()));
    assert_snapshot!("grid_group", snapshot_attrs(&api.grid_group_attrs()));
    assert_snapshot!("head_row", snapshot_attrs(&api.head_row_attrs()));
    assert_snapshot!(
        "head_cell",
        snapshot_attrs(&api.head_cell_attrs(Weekday::Sunday)),
    );
    assert_snapshot!("row", snapshot_attrs(&api.row_attrs(0)));
}

#[test]
fn snapshot_cell_branches() {
    let unavailable = Callback::new_ref(|date: &CalendarDate| date.day() == 5);

    let mut svc = service_with(
        props()
            .today(date(2024, 1, 15))
            .min(Some(date(2024, 1, 3)))
            .is_date_unavailable(Some(unavailable)),
        en_us(),
    );

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 10),
    }));
    drop(svc.send(Event::HoverDate {
        date: date(2024, 1, 12),
    }));

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "cell_anchor",
        snapshot_attrs(&api.cell_attrs(&date(2024, 1, 10)))
    );
    assert_snapshot!(
        "cell_trigger_anchor",
        snapshot_attrs(&api.cell_trigger_attrs(&date(2024, 1, 10))),
    );
    assert_snapshot!(
        "cell_trigger_hover_range",
        snapshot_attrs(&api.cell_trigger_attrs(&date(2024, 1, 11))),
    );
    assert_snapshot!(
        "cell_trigger_disabled",
        snapshot_attrs(&api.cell_trigger_attrs(&date(2024, 1, 1))),
    );
    assert_snapshot!(
        "cell_trigger_unavailable",
        snapshot_attrs(&api.cell_trigger_attrs(&date(2024, 1, 5))),
    );
    assert_snapshot!(
        "cell_trigger_today",
        snapshot_attrs(&api.cell_trigger_attrs(&date(2024, 1, 15))),
    );
}

#[test]
fn snapshot_confirmed_range_branches() {
    let svc = service_with(
        props().value(Some(range(date(2024, 1, 10), date(2024, 1, 12)))),
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "cell_trigger_range_start",
        snapshot_attrs(&api.cell_trigger_attrs(&date(2024, 1, 10))),
    );
    assert_snapshot!(
        "cell_trigger_in_range",
        snapshot_attrs(&api.cell_trigger_attrs(&date(2024, 1, 11))),
    );
    assert_snapshot!(
        "cell_trigger_range_end",
        snapshot_attrs(&api.cell_trigger_attrs(&date(2024, 1, 12))),
    );
}
