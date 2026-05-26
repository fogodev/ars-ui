//! Unit and snapshot tests for the `Calendar` component.
//!
//! Test names that begin with `snapshot_` use `insta::assert_snapshot!` and
//! commit golden output under `snapshots/`. Every other test is a pure
//! state-machine assertion that does not depend on `.snap` files.

use alloc::{format, string::String, sync::Arc, vec::Vec};

use ars_core::{AttrMap, Callback, ComponentPart, Env, HtmlAttr, KeyboardKey, Service};
use ars_i18n::{
    CalendarDate, DateDuration, Locale, StubIntlBackend,
    locales::{ar_sa, de_de, en_gb, en_us, fa},
};
use insta::assert_snapshot;

use super::*;

// ────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────

fn date(year: i32, month: u8, day: u8) -> CalendarDate {
    CalendarDate::new_gregorian(year, month, day).expect("valid test date")
}

fn props() -> Props {
    Props::new().id("cal").today(date(2024, 1, 15))
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

// ────────────────────────────────────────────────────────────────────
// Initial state
// ────────────────────────────────────────────────────────────────────

#[test]
fn initial_state_is_idle() {
    let svc = service();

    assert_eq!(*svc.state(), State::Idle);
}

#[test]
fn initial_focused_date_falls_back_to_today() {
    let svc = service();

    assert_eq!(svc.context().focused_date, date(2024, 1, 15));
}

#[test]
fn initial_focused_date_prefers_controlled_value() {
    let props = props().value(Some(date(2023, 7, 4)));

    let svc = service_with(props, en_us());

    assert_eq!(svc.context().focused_date, date(2023, 7, 4));
    assert_eq!(svc.context().visible_month, 7);
    assert_eq!(svc.context().visible_year, 2023);
}

#[test]
fn focus_in_transitions_to_focused() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));

    assert_eq!(*svc.state(), State::Focused);
}

#[test]
fn focus_out_returns_to_idle() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));
    drop(svc.send(Event::FocusOut));

    assert_eq!(*svc.state(), State::Idle);
}

// ────────────────────────────────────────────────────────────────────
// Anatomy / connect
// ────────────────────────────────────────────────────────────────────

#[test]
fn grid_has_role_grid() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.grid_attrs(), HtmlAttr::Role).as_deref(),
        Some("grid")
    );
}

#[test]
fn cells_have_role_gridcell_with_aria_selected_when_selected() {
    let chosen = date(2024, 1, 10);

    let props = props().value(Some(chosen.clone()));

    let svc = service_with(props, en_us());

    let api = svc.connect(&|_| {});

    let cell = api.cell_attrs(&chosen);

    assert_eq!(attr(&cell, HtmlAttr::Role).as_deref(), Some("gridcell"));
    assert_eq!(
        attr(&cell, HtmlAttr::Aria(AriaAttr::Selected)).as_deref(),
        Some("true"),
    );

    // A non-selected cell should not carry aria-selected.
    let other = date(2024, 1, 11);

    let cell = api.cell_attrs(&other);

    assert!(cell.get(&HtmlAttr::Aria(AriaAttr::Selected)).is_none());
}

#[test]
fn cell_trigger_aria_label_includes_full_date_and_weekday() {
    let svc = service();

    let api = svc.connect(&|_| {});

    let trigger = api.cell_trigger_attrs(&date(2024, 1, 15));

    let label = attr(&trigger, HtmlAttr::Aria(AriaAttr::Label)).unwrap_or_default();

    assert!(
        label.contains("January"),
        "label {label:?} should contain month name"
    );
    assert!(
        label.contains("15"),
        "label {label:?} should contain day number"
    );
    assert!(
        label.contains("2024"),
        "label {label:?} should contain year"
    );
    assert!(
        label.contains("Monday"),
        "label {label:?} should contain weekday"
    );
}

#[test]
fn prev_next_triggers_have_aria_labels_from_messages() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.prev_trigger_attrs(), HtmlAttr::Aria(AriaAttr::Label)).as_deref(),
        Some("Previous month"),
    );
    assert_eq!(
        attr(&api.next_trigger_attrs(), HtmlAttr::Aria(AriaAttr::Label)).as_deref(),
        Some("Next month"),
    );
}

#[test]
fn prev_next_triggers_use_plural_labels_when_multi_month_visible_step() {
    let props = props()
        .visible_months(3)
        .page_behavior(PageBehavior::Visible);

    let svc = service_with(props, en_us());

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.prev_trigger_attrs(), HtmlAttr::Aria(AriaAttr::Label)).as_deref(),
        Some("Previous 3 months"),
    );
    assert_eq!(
        attr(&api.next_trigger_attrs(), HtmlAttr::Aria(AriaAttr::Label)).as_deref(),
        Some("Next 3 months"),
    );
}

#[test]
fn head_cells_use_scope_col_with_abbr_full_name() {
    let svc = service();

    let api = svc.connect(&|_| {});

    let cell = api.head_cell_attrs(Weekday::Monday);

    assert_eq!(attr(&cell, HtmlAttr::Scope).as_deref(), Some("col"));
    assert_eq!(attr(&cell, HtmlAttr::Abbr).as_deref(), Some("Monday"));
}

#[test]
fn heading_has_aria_live_polite_and_atomic_true() {
    let svc = service();

    let api = svc.connect(&|_| {});

    let h = api.heading_attrs();

    assert_eq!(
        attr(&h, HtmlAttr::Aria(AriaAttr::Live)).as_deref(),
        Some("polite"),
    );
    assert_eq!(
        attr(&h, HtmlAttr::Aria(AriaAttr::Atomic)).as_deref(),
        Some("true"),
    );
}

// ────────────────────────────────────────────────────────────────────
// i18n
// ────────────────────────────────────────────────────────────────────

#[test]
fn week_day_labels_start_sunday_for_en_us() {
    let svc = service();

    let labels = svc.context().week_day_labels();

    assert_eq!(labels.first().map(|(wd, _)| *wd), Some(Weekday::Sunday));
    assert_eq!(labels.first().map(|(_, label)| label.as_str()), Some("Su"));
}

#[test]
fn week_day_labels_start_monday_for_en_gb() {
    let svc = service_with(props(), en_gb());

    let labels = svc.context().week_day_labels();

    assert_eq!(labels.first().map(|(wd, _)| *wd), Some(Weekday::Monday));
}

#[test]
fn first_day_of_week_prop_overrides_locale() {
    let svc = service_with(props().first_day_of_week(Some(Weekday::Wednesday)), en_us());

    let labels = svc.context().week_day_labels();

    assert_eq!(labels.first().map(|(wd, _)| *wd), Some(Weekday::Wednesday));
}

#[test]
fn first_day_of_week_defaults_to_saturday_for_ar_sa() {
    let svc = service_with(props(), ar_sa());

    assert_eq!(svc.context().first_day_of_week, Weekday::Saturday);
}

#[test]
fn first_day_of_week_defaults_to_monday_for_de_de() {
    let svc = service_with(props(), de_de());

    assert_eq!(svc.context().first_day_of_week, Weekday::Monday);
}

// ────────────────────────────────────────────────────────────────────
// Navigation
// ────────────────────────────────────────────────────────────────────

#[test]
fn next_month_advances_visible_month() {
    let mut svc = service();

    assert_eq!(svc.context().visible_month, 1);

    drop(svc.send(Event::NextMonth));

    assert_eq!(svc.context().visible_month, 2);
}

#[test]
fn prev_month_retreats_visible_month() {
    let mut svc = service();

    drop(svc.send(Event::PrevMonth));

    assert_eq!(svc.context().visible_month, 12);
    assert_eq!(svc.context().visible_year, 2023);
}

#[test]
fn next_year_advances_by_twelve_months() {
    let mut svc = service();

    drop(svc.send(Event::NextYear));

    assert_eq!(svc.context().visible_year, 2025);
    assert_eq!(svc.context().visible_month, 1);
}

#[test]
fn prev_year_retreats_by_twelve_months() {
    let mut svc = service();

    drop(svc.send(Event::PrevYear));

    assert_eq!(svc.context().visible_year, 2023);
    assert_eq!(svc.context().visible_month, 1);
}

#[test]
fn page_behavior_visible_advances_by_visible_months_count() {
    let mut svc = service_with(
        props()
            .visible_months(3)
            .page_behavior(PageBehavior::Visible),
        en_us(),
    );

    drop(svc.send(Event::NextMonth));

    assert_eq!(svc.context().visible_month, 4);
}

#[test]
fn page_behavior_single_advances_by_one_month_even_when_multi_visible() {
    let mut svc = service_with(
        props()
            .visible_months(3)
            .page_behavior(PageBehavior::Single),
        en_us(),
    );

    drop(svc.send(Event::NextMonth));

    assert_eq!(svc.context().visible_month, 2);
}

#[test]
fn set_month_clamps_to_1_through_12() {
    let mut svc = service();

    drop(svc.send(Event::SetMonth { month: 0 }));

    assert_eq!(svc.context().visible_month, 1);

    drop(svc.send(Event::SetMonth { month: 13 }));

    assert_eq!(svc.context().visible_month, 1);

    drop(svc.send(Event::SetMonth { month: 7 }));

    assert_eq!(svc.context().visible_month, 7);
}

#[test]
fn navigation_emits_announce_month_effect() {
    let mut svc = service();

    let result = svc.send(Event::NextMonth);

    assert!(
        result
            .pending_effects
            .iter()
            .any(|e| e.name == Effect::AnnounceMonth),
        "next-month should queue AnnounceMonth effect",
    );
}

// ────────────────────────────────────────────────────────────────────
// Keyboard
// ────────────────────────────────────────────────────────────────────

#[test]
fn arrow_right_moves_focused_date_by_one_day() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));

    let before = svc.context().focused_date.clone();

    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::ArrowRight,
        shift: false,
    }));

    assert_eq!(svc.context().focused_date, before.add_days(1).unwrap(),);
}

#[test]
fn arrow_left_moves_focused_date_back_one_day() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));

    let before = svc.context().focused_date.clone();

    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::ArrowLeft,
        shift: false,
    }));

    assert_eq!(svc.context().focused_date, before.add_days(-1).unwrap());
}

#[test]
fn arrow_down_moves_focused_date_by_one_week() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));

    let before = svc.context().focused_date.clone();

    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::ArrowDown,
        shift: false,
    }));

    assert_eq!(svc.context().focused_date, before.add_days(7).unwrap());
}

#[test]
fn arrow_keys_swap_in_rtl_locale() {
    let mut svc = service_with(props().is_rtl(true), fa());

    drop(svc.send(Event::FocusIn));

    let before = svc.context().focused_date.clone();

    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::ArrowLeft,
        shift: false,
    }));

    // RTL: ArrowLeft means "next day".
    assert_eq!(svc.context().focused_date, before.add_days(1).unwrap());
}

#[test]
fn enter_selects_focused_date() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));

    let target = svc.context().focused_date.clone();

    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::Enter,
        shift: false,
    }));

    assert_eq!(*svc.context().value.get(), Some(target));
}

#[test]
fn space_selects_focused_date() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));

    let target = svc.context().focused_date.clone();

    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::Space,
        shift: false,
    }));

    assert_eq!(*svc.context().value.get(), Some(target));
}

#[test]
fn page_up_changes_month_back() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));
    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::PageUp,
        shift: false,
    }));

    // Focused date moves back one month; visible month follows via sync.
    assert_eq!(svc.context().focused_date, date(2023, 12, 15));
    assert_eq!(svc.context().visible_month, 12);
    assert_eq!(svc.context().visible_year, 2023);
}

#[test]
fn page_down_changes_month_forward() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));
    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::PageDown,
        shift: false,
    }));

    assert_eq!(svc.context().focused_date, date(2024, 2, 15));
    assert_eq!(svc.context().visible_month, 2);
}

#[test]
fn shift_page_down_navigates_year_forward() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));

    let before_year = svc.context().visible_year;

    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::PageDown,
        shift: true,
    }));

    assert_eq!(svc.context().visible_year, before_year + 1);
}

#[test]
fn shift_page_up_navigates_year_back() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));

    let before_year = svc.context().visible_year;

    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::PageUp,
        shift: true,
    }));

    assert_eq!(svc.context().visible_year, before_year - 1);
}

#[test]
fn home_moves_focus_to_start_of_current_week() {
    // Today is Jan 15, 2024 (Monday). With Sunday-start, "Home" should
    // go to Jan 14 (Sunday).
    let mut svc = service();

    drop(svc.send(Event::FocusIn));
    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::Home,
        shift: false,
    }));

    assert_eq!(svc.context().focused_date, date(2024, 1, 14));
}

#[test]
fn end_moves_focus_to_end_of_current_week() {
    // Sunday-start, Jan 15 is Monday → end of week is Jan 20 (Saturday).
    let mut svc = service();

    drop(svc.send(Event::FocusIn));
    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::End,
        shift: false,
    }));

    assert_eq!(svc.context().focused_date, date(2024, 1, 20));
}

#[test]
fn arrow_into_next_month_auto_scrolls_visible_month() {
    // Jan 31 + 1 day → Feb 1; the calendar should follow.
    let mut svc = service_with(props().today(date(2024, 1, 31)), en_us());

    drop(svc.send(Event::FocusIn));
    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::ArrowRight,
        shift: false,
    }));

    assert_eq!(svc.context().focused_date, date(2024, 2, 1));
    assert_eq!(svc.context().visible_month, 2);
}

// ────────────────────────────────────────────────────────────────────
// Constraints
// ────────────────────────────────────────────────────────────────────

#[test]
fn min_max_disable_out_of_range_cells_with_aria_disabled() {
    let svc = service_with(
        props()
            .min(Some(date(2024, 1, 10)))
            .max(Some(date(2024, 1, 20))),
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let outside = api.cell_attrs(&date(2024, 1, 5));

    assert_eq!(
        attr(&outside, HtmlAttr::Aria(AriaAttr::Disabled)).as_deref(),
        Some("true"),
    );

    let inside = api.cell_attrs(&date(2024, 1, 15));

    assert!(inside.get(&HtmlAttr::Aria(AriaAttr::Disabled)).is_none());
}

#[test]
fn focus_date_clamps_to_min_max() {
    let mut svc = service_with(
        props()
            .min(Some(date(2024, 1, 10)))
            .max(Some(date(2024, 1, 20))),
        en_us(),
    );

    drop(svc.send(Event::FocusDate {
        date: date(2024, 1, 1),
    }));

    assert_eq!(svc.context().focused_date, date(2024, 1, 10));
}

#[test]
fn prev_disabled_when_first_visible_month_at_or_before_min() {
    // visible_month = 1 (Jan 2024); min = Feb 1 2024 → prev should be disabled
    // because the first of the visible month (Jan 1) is <= min.
    let svc = service_with(props().min(Some(date(2024, 2, 1))), en_us());

    let api = svc.connect(&|_| {});

    assert!(api.is_prev_disabled());
}

#[test]
fn next_disabled_when_last_visible_month_at_or_after_max() {
    let svc = service_with(props().max(Some(date(2024, 1, 25))), en_us());

    let api = svc.connect(&|_| {});

    assert!(api.is_next_disabled());
}

#[test]
fn disabled_calendar_blocks_select_event() {
    let mut svc = service_with(props().disabled(true), en_us());

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 20),
    }));

    assert_eq!(*svc.context().value.get(), None);
}

#[test]
fn readonly_allows_focus_but_blocks_select() {
    let mut svc = service_with(props().readonly(true), en_us());

    drop(svc.send(Event::FocusIn));

    assert_eq!(*svc.state(), State::Focused);

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 20),
    }));

    assert_eq!(*svc.context().value.get(), None);
}

#[test]
fn unavailable_predicate_marks_cells_with_data_ars_unavailable_and_aria_disabled() {
    let is_unavailable = Callback::new_ref(|date: &CalendarDate| date.day() == 12);

    let svc = service_with(props().is_date_unavailable(Some(is_unavailable)), en_us());

    let api = svc.connect(&|_| {});

    let trigger = api.cell_trigger_attrs(&date(2024, 1, 12));

    assert_eq!(
        attr(&trigger, HtmlAttr::Data("ars-unavailable")).as_deref(),
        Some("true"),
    );
    assert_eq!(
        attr(&trigger, HtmlAttr::Aria(AriaAttr::Disabled)).as_deref(),
        Some("true"),
    );

    let label = attr(&trigger, HtmlAttr::Aria(AriaAttr::Label)).unwrap_or_default();

    assert!(label.ends_with("(unavailable)"), "label was {label:?}");
}

// ────────────────────────────────────────────────────────────────────
// Today
// ────────────────────────────────────────────────────────────────────

#[test]
fn today_cell_has_data_ars_today_attribute() {
    let svc = service();

    let api = svc.connect(&|_| {});

    let trigger = api.cell_trigger_attrs(&date(2024, 1, 15));

    assert_eq!(
        attr(&trigger, HtmlAttr::Data("ars-today")).as_deref(),
        Some("true"),
    );

    let other = api.cell_trigger_attrs(&date(2024, 1, 16));

    assert!(other.get(&HtmlAttr::Data("ars-today")).is_none());
}

// ────────────────────────────────────────────────────────────────────
// Multi-select (§5)
// ────────────────────────────────────────────────────────────────────

#[test]
fn selection_mode_multiple_uses_toggle_date() {
    let mut svc = service_with(props().selection_mode(SelectionMode::Multiple), en_us());

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 10),
    }));

    assert!(
        svc.context()
            .selected_dates
            .get()
            .contains(&date(2024, 1, 10))
    );
}

#[test]
fn toggle_date_adds_then_removes() {
    let mut svc = service_with(props().selection_mode(SelectionMode::Multiple), en_us());

    drop(svc.send(Event::ToggleDate {
        date: date(2024, 1, 10),
    }));
    drop(svc.send(Event::ToggleDate {
        date: date(2024, 1, 12),
    }));

    assert_eq!(svc.context().selected_dates.get().len(), 2);
    drop(svc.send(Event::ToggleDate {
        date: date(2024, 1, 10),
    }));

    assert_eq!(svc.context().selected_dates.get().len(), 1);
    assert!(
        !svc.context()
            .selected_dates
            .get()
            .contains(&date(2024, 1, 10))
    );
}

#[test]
fn max_selected_blocks_excess_toggles_silently() {
    let mut svc = service_with(
        props()
            .selection_mode(SelectionMode::Multiple)
            .max_selected(Some(2)),
        en_us(),
    );

    drop(svc.send(Event::ToggleDate {
        date: date(2024, 1, 10),
    }));
    drop(svc.send(Event::ToggleDate {
        date: date(2024, 1, 11),
    }));
    drop(svc.send(Event::ToggleDate {
        date: date(2024, 1, 12),
    }));

    let set = svc.context().selected_dates.get();

    assert_eq!(set.len(), 2);
    assert!(!set.contains(&date(2024, 1, 12)));
}

#[test]
fn multi_select_grid_has_aria_multiselectable_true() {
    let svc = service_with(props().selection_mode(SelectionMode::Multiple), en_us());

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.grid_attrs(), HtmlAttr::Aria(AriaAttr::MultiSelectable)).as_deref(),
        Some("true"),
    );
}

#[test]
fn single_select_grid_does_not_have_aria_multiselectable() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert!(
        api.grid_attrs()
            .get(&HtmlAttr::Aria(AriaAttr::MultiSelectable))
            .is_none()
    );
}

#[test]
fn multi_select_cell_aria_selected_reflects_set_membership() {
    let svc = service_with(
        props()
            .selection_mode(SelectionMode::Multiple)
            .default_selected_dates(SelectedDates::from_iter([date(2024, 1, 11)])),
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let selected = api.cell_attrs(&date(2024, 1, 11));

    assert_eq!(
        attr(&selected, HtmlAttr::Aria(AriaAttr::Selected)).as_deref(),
        Some("true"),
    );
}

// ────────────────────────────────────────────────────────────────────
// Multi-month layout
// ────────────────────────────────────────────────────────────────────

#[test]
fn weeks_for_returns_correct_month_per_offset() {
    let svc = service_with(props().visible_months(2), en_us());

    let weeks0 = svc.context().weeks_for(0);
    let weeks1 = svc.context().weeks_for(1);

    assert_eq!(weeks0[2][3].month(), 1);
    assert_eq!(weeks1[2][3].month(), 2);
}

#[test]
fn sync_visible_does_not_scroll_when_focus_stays_in_range() {
    let mut svc = service_with(props().visible_months(2), en_us());

    drop(svc.send(Event::FocusDate {
        date: date(2024, 2, 10),
    }));

    assert_eq!(svc.context().visible_month, 1);
}

#[test]
fn range_heading_text_uses_separator_for_multi_month() {
    let svc = service_with(props().visible_months(2), en_us());

    let api = svc.connect(&|_| {});

    let heading = api.range_heading_text();

    assert!(
        heading.contains('\u{2013}'),
        "expected en-dash in {heading:?}"
    );
    assert!(
        heading.contains("January"),
        "first month label missing: {heading:?}"
    );
    assert!(
        heading.contains("February"),
        "last month label missing: {heading:?}"
    );
}

#[test]
fn grid_group_only_makes_sense_for_multi_month() {
    let svc = service_with(props().visible_months(2), en_us());

    let api = svc.connect(&|_| {});

    let group = api.grid_group_attrs();

    assert_eq!(attr(&group, HtmlAttr::Role).as_deref(), Some("group"));
}

#[test]
fn cell_attrs_for_offset_marks_outside_for_other_grids_month() {
    let svc = service_with(props().visible_months(2), en_us());

    let api = svc.connect(&|_| {});

    // From the perspective of offset=1 (February), Jan 15 is outside.
    let cell = api.cell_attrs_for(&date(2024, 1, 15), 1);

    assert_eq!(
        attr(&cell, HtmlAttr::Data("ars-outside-month")).as_deref(),
        Some("true"),
    );
}

// ────────────────────────────────────────────────────────────────────
// Spec-conformance: Part enum order matches the anatomy table.
// ────────────────────────────────────────────────────────────────────

#[test]
fn part_anatomy_matches_spec() {
    assert_eq!(Part::scope(), "calendar");

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

// ────────────────────────────────────────────────────────────────────
// Snapshot tests
// ────────────────────────────────────────────────────────────────────

#[test]
fn snapshot_root_idle() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_idle", snapshot_attrs(&api.root_attrs()));
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
fn snapshot_root_focused() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_focused", snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_header() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("header", snapshot_attrs(&api.header_attrs()));
}

#[test]
fn snapshot_prev_trigger_default() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "prev_trigger_default",
        snapshot_attrs(&api.prev_trigger_attrs())
    );
}

#[test]
fn snapshot_next_trigger_default() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "next_trigger_default",
        snapshot_attrs(&api.next_trigger_attrs())
    );
}

#[test]
fn snapshot_prev_trigger_disabled_by_min() {
    let svc = service_with(props().min(Some(date(2024, 2, 1))), en_us());

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "prev_trigger_disabled_by_min",
        snapshot_attrs(&api.prev_trigger_attrs()),
    );
}

#[test]
fn snapshot_next_trigger_disabled_by_max() {
    let svc = service_with(props().max(Some(date(2024, 1, 25))), en_us());

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "next_trigger_disabled_by_max",
        snapshot_attrs(&api.next_trigger_attrs()),
    );
}

#[test]
fn snapshot_prev_next_trigger_multi_month_visible_step() {
    let svc = service_with(
        props()
            .visible_months(2)
            .page_behavior(PageBehavior::Visible),
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "prev_trigger_multi_month_visible_step",
        snapshot_attrs(&api.prev_trigger_attrs()),
    );
    assert_snapshot!(
        "next_trigger_multi_month_visible_step",
        snapshot_attrs(&api.next_trigger_attrs()),
    );
}

#[test]
fn snapshot_heading() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("heading", snapshot_attrs(&api.heading_attrs()));
}

#[test]
fn snapshot_grid_single_select() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("grid_single_select", snapshot_attrs(&api.grid_attrs()));
}

#[test]
fn snapshot_grid_multi_select() {
    let svc = service_with(props().selection_mode(SelectionMode::Multiple), en_us());

    let api = svc.connect(&|_| {});

    assert_snapshot!("grid_multi_select", snapshot_attrs(&api.grid_attrs()));
}

#[test]
fn snapshot_grid_group_multi_month() {
    let svc = service_with(props().visible_months(2), en_us());

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "grid_group_multi_month",
        snapshot_attrs(&api.grid_group_attrs())
    );
}

#[test]
fn snapshot_head_row() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("head_row", snapshot_attrs(&api.head_row_attrs()));
}

#[test]
fn snapshot_head_cell_sunday() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "head_cell_sunday",
        snapshot_attrs(&api.head_cell_attrs(Weekday::Sunday))
    );
}

#[test]
fn snapshot_head_cell_monday() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "head_cell_monday",
        snapshot_attrs(&api.head_cell_attrs(Weekday::Monday))
    );
}

#[test]
fn snapshot_row() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("row", snapshot_attrs(&api.row_attrs(0)));
}

#[test]
fn snapshot_cell_selected() {
    let chosen = date(2024, 1, 15);

    let svc = service_with(props().value(Some(chosen.clone())), en_us());

    let api = svc.connect(&|_| {});

    assert_snapshot!("cell_selected", snapshot_attrs(&api.cell_attrs(&chosen)));
}

#[test]
fn snapshot_cell_outside_month() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "cell_outside_month",
        snapshot_attrs(&api.cell_attrs(&date(2023, 12, 31))),
    );
}

#[test]
fn snapshot_cell_disabled_by_min() {
    let svc = service_with(props().min(Some(date(2024, 1, 20))), en_us());

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "cell_disabled_by_min",
        snapshot_attrs(&api.cell_attrs(&date(2024, 1, 5))),
    );
}

#[test]
fn snapshot_cell_trigger_focused() {
    let svc = service();

    let api = svc.connect(&|_| {});

    let trigger = api.cell_trigger_attrs(&date(2024, 1, 15));

    assert_snapshot!("cell_trigger_focused", snapshot_attrs(&trigger));
}

#[test]
fn snapshot_cell_trigger_unfocused() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "cell_trigger_unfocused",
        snapshot_attrs(&api.cell_trigger_attrs(&date(2024, 1, 20))),
    );
}

#[test]
fn snapshot_cell_trigger_selected() {
    let svc = service_with(
        props()
            .value(Some(date(2024, 1, 20)))
            .today(date(2024, 1, 15)),
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "cell_trigger_selected",
        snapshot_attrs(&api.cell_trigger_attrs(&date(2024, 1, 20))),
    );
}

#[test]
fn snapshot_cell_trigger_disabled_and_unavailable() {
    let predicate = Callback::new_ref(|d: &CalendarDate| d.day() == 5);

    let svc = service_with(
        props()
            .min(Some(date(2024, 1, 10)))
            .is_date_unavailable(Some(predicate)),
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "cell_trigger_disabled_and_unavailable",
        snapshot_attrs(&api.cell_trigger_attrs(&date(2024, 1, 5))),
    );
}

#[test]
fn snapshot_cell_trigger_today() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "cell_trigger_today",
        snapshot_attrs(&api.cell_trigger_attrs(&date(2024, 1, 15))),
    );
}

// ────────────────────────────────────────────────────────────────────
// Additional coverage — SelectedDates, Props builder, transitions
// ────────────────────────────────────────────────────────────────────

#[test]
fn selected_dates_set_semantics() {
    let mut set = SelectedDates::new();

    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
    assert!(set.insert(date(2024, 1, 10)));
    assert!(!set.insert(date(2024, 1, 10)));
    assert!(set.insert(date(2024, 1, 12)));
    assert!(set.contains(&date(2024, 1, 10)));
    assert!(!set.is_empty());
    assert_eq!(set.len(), 2);

    let slice = set.as_slice();

    assert_eq!(slice.len(), 2);
    assert!(set.remove(&date(2024, 1, 10)));
    assert!(!set.remove(&date(2024, 1, 10)));

    let collected: Vec<&CalendarDate> = (&set).into_iter().collect();

    assert_eq!(collected.len(), 1);

    let collected: SelectedDates = [date(2024, 2, 1), date(2024, 2, 2)].into_iter().collect();

    assert_eq!(collected.len(), 2);
}

#[test]
fn props_equality_handles_callback_pointer_identity() {
    let base = props();
    let same = props();

    assert_eq!(base, same);

    let cb_a = Callback::new_ref(|_: &CalendarDate| false);
    let cb_b = Callback::new_ref(|_: &CalendarDate| true);

    let with_a = props().is_date_unavailable(Some(cb_a.clone()));
    let with_b = props().is_date_unavailable(Some(cb_b));

    assert_ne!(with_a, with_b);

    // Same callback instance compares equal even when cloned (Arc::ptr_eq).
    let with_a_again = props().is_date_unavailable(Some(cb_a));

    assert_eq!(with_a, with_a_again);
}

#[test]
fn props_builders_round_trip_all_fields() {
    let configured = Props::new()
        .id("cal-2")
        .value(Some(date(2024, 6, 1)))
        .default_value(Some(date(2024, 6, 15)))
        .selected_dates(Some(SelectedDates::from_iter([date(2024, 6, 5)])))
        .default_selected_dates(SelectedDates::from_iter([date(2024, 6, 6)]))
        .selection_mode(SelectionMode::Multiple)
        .max_selected(Some(4))
        .min(Some(date(2024, 1, 1)))
        .max(Some(date(2024, 12, 31)))
        .disabled(false)
        .readonly(false)
        .first_day_of_week(Some(Weekday::Tuesday))
        .show_week_numbers(true)
        .is_rtl(true)
        .visible_months(2)
        .page_behavior(PageBehavior::Single)
        .today(date(2024, 6, 10));

    assert_eq!(configured.id, "cal-2");
    assert_eq!(configured.value, Some(Some(date(2024, 6, 1))));
    assert_eq!(configured.default_value, Some(date(2024, 6, 15)));
    assert_eq!(configured.selection_mode, SelectionMode::Multiple);
    assert_eq!(configured.max_selected, Some(4));
    assert_eq!(configured.first_day_of_week, Some(Weekday::Tuesday));
    assert!(configured.show_week_numbers);
    assert!(configured.is_rtl);
    assert_eq!(configured.visible_months, 2);
    assert_eq!(configured.page_behavior, PageBehavior::Single);
    assert_eq!(configured.today, date(2024, 6, 10));
}

#[test]
fn props_debug_renders() {
    let formatted = format!("{:?}", props());

    assert!(formatted.contains("Props"));
}

#[test]
fn context_debug_renders() {
    let svc = service();

    let formatted = format!("{:?}", svc.context());

    assert!(formatted.contains("Context"));
    assert!(formatted.contains("focused_date"));
}

#[test]
fn messages_debug_renders() {
    let formatted = format!("{:?}", Messages::default());

    assert!(formatted.contains("Messages"));
}

#[test]
fn messages_clone_and_equality() {
    let messages = Messages::default();

    let cloned = messages.clone();

    assert_eq!(messages, cloned);
}

#[test]
fn api_debug_renders() {
    let svc = service();

    let api = svc.connect(&|_| {});

    let formatted = format!("{api:?}");

    assert!(formatted.contains("Api"));
}

#[test]
fn focus_date_clamps_to_max() {
    let mut svc = service_with(
        props()
            .min(Some(date(2024, 1, 10)))
            .max(Some(date(2024, 1, 20))),
        en_us(),
    );

    drop(svc.send(Event::FocusDate {
        date: date(2024, 1, 31),
    }));

    assert_eq!(svc.context().focused_date, date(2024, 1, 20));
}

#[test]
fn set_year_advances_context() {
    let mut svc = service();

    let result = svc.send(Event::SetYear { year: 2030 });

    assert_eq!(svc.context().visible_year, 2030);
    assert!(
        result
            .pending_effects
            .iter()
            .any(|e| e.name == Effect::AnnounceMonth)
    );
}

#[test]
fn readonly_blocks_multi_toggle() {
    let mut svc = service_with(
        props()
            .selection_mode(SelectionMode::Multiple)
            .readonly(true),
        en_us(),
    );

    drop(svc.send(Event::ToggleDate {
        date: date(2024, 1, 10),
    }));

    assert!(svc.context().selected_dates.get().is_empty());
}

#[test]
fn select_date_skips_disabled_and_unavailable_dates() {
    let never_available = Callback::new_ref(|_: &CalendarDate| true);

    let mut svc = service_with(props().is_date_unavailable(Some(never_available)), en_us());

    drop(svc.send(Event::SelectDate {
        date: date(2024, 1, 10),
    }));

    assert_eq!(*svc.context().value.get(), None);

    let mut svc2 = service_with(props().min(Some(date(2024, 1, 20))), en_us());

    drop(svc2.send(Event::SelectDate {
        date: date(2024, 1, 5),
    }));

    assert_eq!(*svc2.context().value.get(), None);
}

#[test]
fn keydown_enter_in_multi_mode_uses_toggle_event() {
    let mut svc = service_with(props().selection_mode(SelectionMode::Multiple), en_us());

    drop(svc.send(Event::FocusIn));
    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::Enter,
        shift: false,
    }));

    let focused = svc.context().focused_date.clone();

    assert!(svc.context().selected_dates.get().contains(&focused));
}

#[test]
fn keydown_unhandled_key_is_noop() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));

    let before = svc.context().focused_date.clone();

    let result = svc.send(Event::KeyDown {
        key: KeyboardKey::Escape,
        shift: false,
    });

    assert_eq!(svc.context().focused_date, before);
    assert!(!result.state_changed);
    assert!(!result.context_changed);
}

#[test]
fn controlled_multi_select_uses_controlled_bindable() {
    let svc = service_with(
        props()
            .selection_mode(SelectionMode::Multiple)
            .selected_dates(Some(SelectedDates::from_iter([date(2024, 1, 11)]))),
        en_us(),
    );

    assert!(svc.context().selected_dates.is_controlled());
    assert!(
        svc.context()
            .selected_dates
            .get()
            .contains(&date(2024, 1, 11))
    );
}

#[test]
fn controlled_single_select_uses_controlled_bindable() {
    let svc = service_with(props().value(Some(date(2024, 7, 4))), en_us());

    assert!(svc.context().value.is_controlled());
}

#[test]
fn visible_months_zero_clamped_to_one_in_init() {
    let svc = service_with(props().visible_months(0), en_us());

    assert_eq!(svc.context().visible_months, 1);
}

#[test]
fn weeks_helper_returns_six_rows() {
    let svc = service();

    assert_eq!(svc.context().weeks().len(), 6);
}

#[test]
fn is_in_visible_range_helper_reports_first_visible_month() {
    let svc = service();

    assert!(
        svc.context()
            .is_in_visible_range(&svc.context().focused_date)
    );
}

#[test]
fn heading_text_single_month_uses_long_month_name() {
    let svc = service();

    let api = svc.connect(&|_| {});

    let heading = api.heading_text();

    assert_eq!(heading, "January 2024");
}

#[test]
fn grid_attrs_for_uses_offset_specific_ids() {
    let svc = service_with(props().visible_months(2), en_us());

    let api = svc.connect(&|_| {});

    let g0 = api.grid_attrs_for(0);
    let g1 = api.grid_attrs_for(1);

    assert_eq!(attr(&g0, HtmlAttr::Id).as_deref(), Some("cal-grid-0"));
    assert_eq!(attr(&g1, HtmlAttr::Id).as_deref(), Some("cal-grid-1"));
}

#[test]
fn heading_attrs_for_omits_aria_live() {
    let svc = service_with(props().visible_months(2), en_us());

    let api = svc.connect(&|_| {});

    let h = api.heading_attrs_for(1);

    assert!(h.get(&HtmlAttr::Aria(AriaAttr::Live)).is_none());
    assert!(
        attr(&h, HtmlAttr::Id)
            .as_deref()
            .is_some_and(|id| id.ends_with("-heading-1"))
    );
}

#[test]
fn grid_attrs_readonly_and_disabled_paths() {
    let svc_readonly = service_with(props().readonly(true), en_us());

    let api = svc_readonly.connect(&|_| {});

    assert_eq!(
        attr(&api.grid_attrs(), HtmlAttr::Aria(AriaAttr::ReadOnly)).as_deref(),
        Some("true"),
    );

    let svc_disabled = service_with(props().disabled(true), en_us());

    let api = svc_disabled.connect(&|_| {});

    assert_eq!(
        attr(&api.grid_attrs(), HtmlAttr::Aria(AriaAttr::Disabled)).as_deref(),
        Some("true"),
    );
}

#[test]
fn api_view_helpers_expose_visible_layout() {
    let svc = service_with(props().visible_months(3).show_week_numbers(true), en_us());

    let api = svc.connect(&|_| {});

    assert_eq!(api.visible_month_count(), 3);
    assert_eq!(api.month_offsets(), 0..3);
    assert!(api.show_week_numbers());
    assert_eq!(api.selection_mode(), SelectionMode::Single);
    assert!(!api.is_focused());
    assert_eq!(api.weeks_for(1).len(), 6);
    assert_eq!(api.weeks().len(), 6);
    assert_eq!(api.week_day_labels().len(), 7);
    assert_eq!(api.today(), &date(2024, 1, 15));
    assert!(!api.is_outside_visible_month(&date(2024, 1, 10)));
}

#[test]
fn api_send_helpers_route_events_to_machine() {
    use core::cell::RefCell;
    let captured: RefCell<Vec<Event>> = RefCell::new(Vec::new());
    let send = |event: Event| captured.borrow_mut().push(event);

    let svc = service_with(props().selection_mode(SelectionMode::Multiple), en_us());

    let api = svc.connect(&send);

    api.on_cell_click(date(2024, 1, 10));
    api.on_grid_focusin();
    api.on_grid_focusout(true);
    api.on_grid_focusout(false);
    api.on_grid_keydown(KeyboardKey::ArrowRight, true);
    api.on_prev_click();
    api.on_next_click();

    let events = captured.borrow();

    assert!(matches!(events[0], Event::ToggleDate { .. }));
    assert!(matches!(events[1], Event::FocusIn));
    assert!(matches!(events[2], Event::FocusOut));
    assert!(matches!(
        events[3],
        Event::KeyDown {
            key: KeyboardKey::ArrowRight,
            shift: true
        }
    ));
    assert!(matches!(events[4], Event::PrevMonth));
    assert!(matches!(events[5], Event::NextMonth));
    assert_eq!(events.len(), 6);
}

#[test]
fn api_on_cell_click_uses_select_in_single_mode() {
    use core::cell::RefCell;
    let captured: RefCell<Vec<Event>> = RefCell::new(Vec::new());
    let send = |event: Event| captured.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&send);

    api.on_cell_click(date(2024, 1, 10));
    assert!(matches!(
        captured.borrow().first(),
        Some(Event::SelectDate { .. })
    ));
}

#[test]
fn disabled_calendar_blocks_navigation_too() {
    let mut svc = service_with(props().disabled(true), en_us());

    let before_month = svc.context().visible_month;

    drop(svc.send(Event::NextMonth));

    assert_eq!(svc.context().visible_month, before_month);
}

// ────────────────────────────────────────────────────────────────────
// Codex review regressions (PR #688)
// ────────────────────────────────────────────────────────────────────

#[test]
fn unavailable_cell_trigger_stays_focusable_and_not_html_disabled() {
    // Codex T1 (P1): unavailable cells must remain focusable per spec §3.
    // HTML `disabled` removes the element from the focus model, so we only
    // surface `aria-disabled` + the data hook for unavailable-but-not-disabled
    // dates; HTML `disabled` is reserved for the min/max-disabled case.
    let predicate = Callback::new_ref(|d: &CalendarDate| d.day() == 12);
    let svc = service_with(props().is_date_unavailable(Some(predicate)), en_us());
    let api = svc.connect(&|_| {});

    let trigger = api.cell_trigger_attrs(&date(2024, 1, 12));

    // aria-disabled is set (semantic restriction is announced).
    assert_eq!(
        attr(&trigger, HtmlAttr::Aria(AriaAttr::Disabled)).as_deref(),
        Some("true"),
    );
    // HTML `disabled` is NOT set — the cell must remain in the focus model.
    assert!(
        trigger.get(&HtmlAttr::Disabled).is_none(),
        "unavailable cell triggers must remain focusable; HTML disabled removes them from tab order",
    );
}

#[test]
fn min_max_disabled_cell_trigger_is_html_disabled() {
    // Sanity check the other branch: dates outside min/max keep both
    // `aria-disabled` and HTML `disabled` set.
    let svc = service_with(props().min(Some(date(2024, 1, 10))), en_us());
    let api = svc.connect(&|_| {});

    let trigger = api.cell_trigger_attrs(&date(2024, 1, 5));

    assert_eq!(
        attr(&trigger, HtmlAttr::Aria(AriaAttr::Disabled)).as_deref(),
        Some("true"),
    );
    assert_eq!(
        trigger.get(&HtmlAttr::Disabled).map(ToString::to_string),
        Some(String::from("true")),
    );
}

#[test]
fn cell_trigger_attrs_for_offset_marks_outside_for_other_grids_month() {
    // Codex T2 (P2): cell_trigger_attrs_for(offset) checks against the grid's
    // own month, not the first visible month.
    let svc = service_with(props().visible_months(2), en_us());
    let api = svc.connect(&|_| {});

    // From offset=1 (February), Jan 15 is outside-month.
    let trigger = api.cell_trigger_attrs_for(&date(2024, 1, 15), 1);
    assert_eq!(
        attr(&trigger, HtmlAttr::Data("ars-outside-month")).as_deref(),
        Some("true"),
    );

    // From offset=0 (January), Jan 15 is inside.
    let trigger = api.cell_trigger_attrs_for(&date(2024, 1, 15), 0);
    assert!(trigger.get(&HtmlAttr::Data("ars-outside-month")).is_none());

    // Feb 15 from offset=1 (Feb's grid) — should NOT be outside-month
    // (which is the regression the bare cell_trigger_attrs has).
    let trigger = api.cell_trigger_attrs_for(&date(2024, 2, 15), 1);
    assert!(
        trigger.get(&HtmlAttr::Data("ars-outside-month")).is_none(),
        "Feb 15 in Feb's grid must not be flagged outside-month",
    );
}

#[test]
fn next_month_advances_focused_date() {
    // Codex T4 (P1): paging keeps focused_date in the visible range so the
    // roving tab target remains rendered.
    let mut svc = service();
    let before = svc.context().focused_date.clone();

    drop(svc.send(Event::NextMonth));

    let expected = before
        .add(DateDuration {
            months: 1,
            ..DateDuration::default()
        })
        .expect("Jan 15 + 1 month is valid");
    assert_eq!(svc.context().focused_date, expected);
    assert!(
        svc.context()
            .is_in_visible_range(&svc.context().focused_date),
        "focused_date must land in the new visible range",
    );
}

#[test]
fn prev_month_retreats_focused_date() {
    let mut svc = service();
    let before = svc.context().focused_date.clone();

    drop(svc.send(Event::PrevMonth));

    let expected = before
        .add(DateDuration {
            months: -1,
            ..DateDuration::default()
        })
        .expect("Jan 15 - 1 month is valid");
    assert_eq!(svc.context().focused_date, expected);
}

#[test]
fn next_year_advances_focused_date_by_twelve_months() {
    // Codex T4 (P1) — year paging.
    let mut svc = service();
    let before = svc.context().focused_date.clone();

    drop(svc.send(Event::NextYear));

    let expected = before
        .add(DateDuration {
            months: 12,
            ..DateDuration::default()
        })
        .expect("Jan 15 + 12 months is valid");
    assert_eq!(svc.context().focused_date, expected);
}

#[test]
fn prev_year_retreats_focused_date_by_twelve_months() {
    let mut svc = service();
    let before = svc.context().focused_date.clone();

    drop(svc.send(Event::PrevYear));

    let expected = before
        .add(DateDuration {
            months: -12,
            ..DateDuration::default()
        })
        .expect("Jan 15 - 12 months is valid");
    assert_eq!(svc.context().focused_date, expected);
}

#[test]
fn shift_page_down_advances_focused_date_by_one_year() {
    // Codex T3 (P2): keyboard year nav also moves focused_date, matching
    // spec §3.2 ("Shift+PageDown: same day, next year").
    let mut svc = service();
    drop(svc.send(Event::FocusIn));
    let before = svc.context().focused_date.clone();

    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::PageDown,
        shift: true,
    }));

    let expected = before
        .add(DateDuration {
            months: 12,
            ..DateDuration::default()
        })
        .expect("Jan 15 + 12 months is valid");
    assert_eq!(svc.context().focused_date, expected);
}

#[test]
fn shift_page_up_retreats_focused_date_by_one_year() {
    let mut svc = service();
    drop(svc.send(Event::FocusIn));
    let before = svc.context().focused_date.clone();

    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::PageUp,
        shift: true,
    }));

    let expected = before
        .add(DateDuration {
            months: -12,
            ..DateDuration::default()
        })
        .expect("Jan 15 - 12 months is valid");
    assert_eq!(svc.context().focused_date, expected);
}

#[test]
fn paging_clamps_focused_date_into_min_max_range() {
    // When paging pushes focused_date past max (or min), clamp it back into
    // the configured range — the roving tab target must remain selectable.
    let mut svc = service_with(
        props()
            .today(date(2024, 1, 15))
            .max(Some(date(2024, 1, 31))),
        en_us(),
    );

    drop(svc.send(Event::NextMonth));

    // Without clamping, focused_date would be Feb 15 (out of range). With
    // clamping, it lands on Jan 31 — the maximum allowed date.
    assert_eq!(svc.context().focused_date, date(2024, 1, 31));
}

#[test]
fn focus_out_processes_even_when_context_becomes_disabled() {
    // Codex T5 (P2): the disabled guard must not swallow FocusOut, otherwise
    // a calendar that becomes disabled mid-focus stays stuck in State::Focused.
    let mut svc = service();
    drop(svc.send(Event::FocusIn));
    assert_eq!(*svc.state(), State::Focused);

    // Simulate the parent flipping `disabled` to true while focused (e.g.,
    // a controlled prop change). The real plumbing for prop propagation is
    // adapter-side; here we mutate the live context directly.
    svc.context_mut().disabled = true;

    drop(svc.send(Event::FocusOut));

    assert_eq!(
        *svc.state(),
        State::Idle,
        "FocusOut must process even when ctx.disabled is true; otherwise the calendar stays stuck in Focused",
    );
}

#[test]
fn focus_in_still_blocked_when_disabled() {
    // The disabled guard still gates interaction events including FocusIn —
    // a disabled calendar should not be enterable via focus.
    let mut svc = service_with(props().disabled(true), en_us());
    drop(svc.send(Event::FocusIn));
    assert_eq!(*svc.state(), State::Idle);
}

// ────────────────────────────────────────────────────────────────────
// Codex review regressions, pass 2 (PR #688)
// ────────────────────────────────────────────────────────────────────

#[test]
fn initial_focused_date_is_clamped_into_min_max_range() {
    // Codex N1 (P1): the initial focused_date must respect min/max so the
    // roving tab target never lands on a disabled cell.
    let svc = service_with(
        props()
            .today(date(2024, 1, 1))
            .min(Some(date(2024, 1, 10)))
            .max(Some(date(2024, 1, 25))),
        en_us(),
    );
    assert_eq!(svc.context().focused_date, date(2024, 1, 10));

    // Symmetric: today above max → clamped down to max.
    let svc = service_with(
        props()
            .today(date(2024, 1, 31))
            .min(Some(date(2024, 1, 10)))
            .max(Some(date(2024, 1, 25))),
        en_us(),
    );
    assert_eq!(svc.context().focused_date, date(2024, 1, 25));
}

#[test]
fn sync_props_propagates_controlled_value_changes() {
    // Codex N2 (P1): `set_props` must surface parent prop changes —
    // controlled value, disabled, min/max, etc. — into the live context.
    let mut svc = service_with(props().value(Some(date(2024, 1, 10))), en_us());
    assert_eq!(*svc.context().value.get(), Some(date(2024, 1, 10)));

    // Parent flips the controlled value.
    drop(svc.set_props(props().value(Some(date(2024, 1, 20)))));
    assert_eq!(
        *svc.context().value.get(),
        Some(date(2024, 1, 20)),
        "controlled value change must flow through set_props",
    );
}

#[test]
fn sync_props_propagates_disabled_and_readonly() {
    let mut svc = service();
    assert!(!svc.context().disabled);
    assert!(!svc.context().readonly);

    drop(svc.set_props(props().disabled(true).readonly(true)));

    assert!(
        svc.context().disabled,
        "disabled must propagate via set_props"
    );
    assert!(
        svc.context().readonly,
        "readonly must propagate via set_props"
    );
}

#[test]
fn sync_props_propagates_min_max() {
    let mut svc = service();
    assert!(svc.context().min.is_none());

    drop(
        svc.set_props(
            props()
                .min(Some(date(2024, 1, 10)))
                .max(Some(date(2024, 1, 25))),
        ),
    );

    assert_eq!(svc.context().min, Some(date(2024, 1, 10)));
    assert_eq!(svc.context().max, Some(date(2024, 1, 25)));
}

#[test]
fn set_month_advances_focused_date() {
    // Codex N3 (P2): SetMonth/SetYear must keep focused_date in range.
    let mut svc = service();
    let before = svc.context().focused_date.clone();

    drop(svc.send(Event::SetMonth { month: 6 }));

    assert_eq!(svc.context().visible_month, 6);
    // Focused date follows the visible month: same day, new month.
    assert_eq!(svc.context().focused_date.month(), 6);
    assert_eq!(svc.context().focused_date.day(), before.day());
}

#[test]
fn set_year_advances_focused_date() {
    let mut svc = service();
    let before_day = svc.context().focused_date.day();
    let before_month = svc.context().focused_date.month();

    drop(svc.send(Event::SetYear { year: 2030 }));

    assert_eq!(svc.context().visible_year, 2030);
    assert_eq!(svc.context().focused_date.year(), 2030);
    assert_eq!(svc.context().focused_date.month(), before_month);
    assert_eq!(svc.context().focused_date.day(), before_day);
}

#[test]
fn prev_next_triggers_disabled_when_calendar_disabled() {
    // Codex N4 (P2): a globally-disabled calendar must mark its prev/next
    // triggers as HTML-disabled too, otherwise the buttons appear active
    // while the machine drops their events.
    let svc = service_with(props().disabled(true), en_us());
    let api = svc.connect(&|_| {});

    let prev = api.prev_trigger_attrs();
    let next = api.next_trigger_attrs();

    assert_eq!(
        prev.get(&HtmlAttr::Disabled).map(ToString::to_string),
        Some(String::from("true")),
        "prev trigger must be HTML-disabled when ctx.disabled is true",
    );
    assert_eq!(
        attr(&prev, HtmlAttr::Aria(AriaAttr::Disabled)).as_deref(),
        Some("true"),
    );
    assert_eq!(
        next.get(&HtmlAttr::Disabled).map(ToString::to_string),
        Some(String::from("true")),
    );
    assert_eq!(
        attr(&next, HtmlAttr::Aria(AriaAttr::Disabled)).as_deref(),
        Some("true"),
    );
}

#[test]
fn weeks_for_returns_six_rows_or_empty_never_partial() {
    // Codex N5 (P2): the contract is "exactly 6 rows or empty on boundary
    // failure", never a partially-built vector. Confirm the happy path
    // produces 6 rows; boundary behaviour is exercised in grid.rs unit
    // tests where we can drive `add_days` failure deterministically.
    let svc = service();
    let weeks = svc.context().weeks();
    assert!(
        weeks.is_empty() || weeks.len() == 6,
        "weeks() must return either 0 or 6 rows, got {}",
        weeks.len(),
    );
    assert_eq!(weeks.len(), 6); // happy path
}

#[test]
fn month_step_plan_keeps_visible_and_focused_in_sync_at_boundary() {
    // Codex N6 (P2): if the focused-date shift fails near a representable
    // boundary, visible_month must NOT advance either — they move
    // atomically.
    //
    // We can't easily hit the calendar boundary in a portable test (the
    // overflow point depends on the ICU calendar engine), but we can pin
    // the happy-path invariant: after paging, focused_date.month() must
    // equal visible_month (within the same calendar year wrap).
    let mut svc = service();
    drop(svc.send(Event::NextMonth));
    assert_eq!(
        svc.context().focused_date.month(),
        svc.context().visible_month,
        "focused_date.month() must track visible_month after paging",
    );
    assert_eq!(
        svc.context().focused_date.year(),
        svc.context().visible_year,
    );
}

// ────────────────────────────────────────────────────────────────────
// Codex review regressions, pass 3 (PR #688)
// ────────────────────────────────────────────────────────────────────

#[test]
fn prev_disabled_considers_page_behavior_single_in_multi_month() {
    // Codex N7 (P2): with visible_months=3 and PageBehavior::Single, the
    // calendar shows Mar-Apr-May. If `min = Mar 15`, paging back by one
    // month (Single) yields Feb-Mar-Apr — which still contains
    // selectable dates (≥ Mar 15 in April). prev must NOT be disabled.
    let svc = service_with(
        props()
            .today(date(2024, 3, 15))
            .visible_months(3)
            .page_behavior(PageBehavior::Single)
            .min(Some(date(2024, 3, 15))),
        en_us(),
    );
    let api = svc.connect(&|_| {});

    assert!(
        !api.is_prev_disabled(),
        "prev must not be disabled when the next page back still has selectable dates",
    );
}

#[test]
fn prev_disabled_when_full_prev_page_below_min() {
    // Conversely, prev IS disabled when the whole prev page would be
    // entirely below min — visible Jan 2024, min = Jan 15, paging back
    // (Single) → Dec 2023 which is fully below min.
    let svc = service_with(
        props()
            .today(date(2024, 1, 15))
            .min(Some(date(2024, 1, 15))),
        en_us(),
    );
    let api = svc.connect(&|_| {});

    assert!(
        api.is_prev_disabled(),
        "prev must be disabled when stepping back yields no selectable dates",
    );
}

#[test]
fn next_disabled_considers_page_behavior_single_in_multi_month() {
    // Codex N8 (P2): symmetric case for next. visible Jan-Feb-Mar with
    // PageBehavior::Single and max = Mar 15. Paging forward by one
    // month (Single) yields Feb-Mar-Apr — Feb still has selectable
    // dates (≤ Mar 15 in February). next must NOT be disabled.
    let svc = service_with(
        props()
            .today(date(2024, 1, 15))
            .visible_months(3)
            .page_behavior(PageBehavior::Single)
            .max(Some(date(2024, 3, 15))),
        en_us(),
    );
    let api = svc.connect(&|_| {});

    assert!(
        !api.is_next_disabled(),
        "next must not be disabled when the next page forward still has selectable dates",
    );
}

#[test]
fn next_disabled_when_full_next_page_above_max() {
    // visible Jan 2024 with max = Jan 25, paging forward (Single) → Feb 2024
    // which is fully above max. next IS disabled.
    let svc = service_with(
        props()
            .today(date(2024, 1, 15))
            .max(Some(date(2024, 1, 25))),
        en_us(),
    );
    let api = svc.connect(&|_| {});

    assert!(
        api.is_next_disabled(),
        "next must be disabled when the next page forward yields no selectable dates",
    );
}

// ────────────────────────────────────────────────────────────────────
// Codex review regressions, pass 4 (PR #688)
// ────────────────────────────────────────────────────────────────────

#[test]
fn sync_props_keeps_focused_date_in_visible_range() {
    // Codex N9 (P1): after `sync_props_into_ctx` clamps focused_date, the
    // visible month/year window must follow, otherwise the focused cell
    // is no longer rendered.
    let mut svc = service_with(props().today(date(2024, 1, 15)), en_us());
    assert_eq!(svc.context().visible_month, 1);

    // Parent installs a new min in July — clamping pushes focused_date
    // to Jul 1, and the visible window must follow.
    drop(svc.set_props(props().today(date(2024, 1, 15)).min(Some(date(2024, 7, 1)))));

    assert_eq!(svc.context().focused_date, date(2024, 7, 1));
    assert!(
        svc.context()
            .is_in_visible_range(&svc.context().focused_date),
        "visible window must follow the clamped focused_date",
    );
}

#[test]
fn sync_props_clears_first_day_of_week_override() {
    // Codex N10 (P2): when the parent flips `first_day_of_week` Some→None
    // the override clears and context returns to the locale-derived value.
    let mut svc = service_with(props().first_day_of_week(Some(Weekday::Wednesday)), en_us());
    assert_eq!(svc.context().first_day_of_week, Weekday::Wednesday);

    // Parent removes the override.
    drop(svc.set_props(props().first_day_of_week(None)));

    // en_us locale default is Sunday — context must return to it.
    assert_eq!(
        svc.context().first_day_of_week,
        Weekday::Sunday,
        "clearing the override must restore the locale default",
    );
}

#[test]
fn set_year_handles_extreme_year_without_overflow() {
    // Codex N11 (P2): `SetYear { year: i32::MIN }` would overflow the
    // delta computation against a positive current visible_year. The
    // transition must drop the event safely instead of panicking in
    // debug or wrapping in release.
    let mut svc = service();
    let visible_before = svc.context().visible_year;
    let focused_before = svc.context().focused_date.clone();

    drop(svc.send(Event::SetYear { year: i32::MIN }));

    // The event is dropped (no overflow). Visible year and focused date
    // remain unchanged.
    assert_eq!(svc.context().visible_year, visible_before);
    assert_eq!(svc.context().focused_date, focused_before);
}

#[test]
fn set_month_is_atomic_with_focused_date() {
    // Codex N12 (P2): SetMonth must commit `visible_month` only after
    // the focused-date shift succeeds. Happy-path invariant: after
    // SetMonth, focused_date.month() always equals visible_month.
    let mut svc = service();
    drop(svc.send(Event::SetMonth { month: 7 }));
    assert_eq!(svc.context().visible_month, 7);
    assert_eq!(svc.context().focused_date.month(), 7);
}

#[test]
fn set_year_is_atomic_with_focused_date() {
    // Codex N13 (P2): SetYear same atomicity rule.
    let mut svc = service();
    drop(svc.send(Event::SetYear { year: 2030 }));
    assert_eq!(svc.context().visible_year, 2030);
    assert_eq!(svc.context().focused_date.year(), 2030);
}

// ────────────────────────────────────────────────────────────────────
// Codex review regressions, pass 5 (PR #688)
// ────────────────────────────────────────────────────────────────────

#[test]
fn paging_pulls_visible_back_when_clamp_moves_focus_to_other_month() {
    // Codex N14 (P1): with focus Jan 15 and max=Jan 25, pressing NextMonth
    // tries to shift to Feb 15, clamps back to Jan 25 (the max). visible_month
    // must follow the clamp — both should land on January.
    let mut svc = service_with(
        props()
            .today(date(2024, 1, 15))
            .max(Some(date(2024, 1, 25))),
        en_us(),
    );
    drop(svc.send(Event::NextMonth));

    assert_eq!(svc.context().focused_date, date(2024, 1, 25));
    assert_eq!(
        svc.context().visible_month,
        svc.context().focused_date.month(),
        "visible_month must follow clamped focused_date",
    );
}

#[test]
fn set_month_pulls_visible_back_when_clamp_moves_focus() {
    // Codex N14 (P1): SetMonth follows the same rule.
    let mut svc = service_with(
        props()
            .today(date(2024, 1, 15))
            .max(Some(date(2024, 1, 25))),
        en_us(),
    );
    drop(svc.send(Event::SetMonth { month: 6 }));

    assert_eq!(
        svc.context().visible_month,
        svc.context().focused_date.month(),
    );
}

#[test]
fn set_year_pulls_visible_back_when_clamp_moves_focus() {
    let mut svc = service_with(
        props()
            .today(date(2024, 1, 15))
            .max(Some(date(2024, 12, 31))),
        en_us(),
    );
    drop(svc.send(Event::SetYear { year: 2030 }));

    assert_eq!(
        svc.context().visible_year,
        svc.context().focused_date.year(),
    );
}

#[test]
fn cell_trigger_carries_button_type() {
    // Codex N15 (P1): cell trigger must declare `type="button"` so a
    // calendar inside a <form> doesn't accidentally submit when a date is
    // clicked.
    let svc = service();
    let api = svc.connect(&|_| {});
    let trigger = api.cell_trigger_attrs(&date(2024, 1, 15));
    assert_eq!(attr(&trigger, HtmlAttr::Type).as_deref(), Some("button"));
}

#[test]
fn prev_next_triggers_carry_button_type() {
    let svc = service();
    let api = svc.connect(&|_| {});
    assert_eq!(
        attr(&api.prev_trigger_attrs(), HtmlAttr::Type).as_deref(),
        Some("button"),
    );
    assert_eq!(
        attr(&api.next_trigger_attrs(), HtmlAttr::Type).as_deref(),
        Some("button"),
    );
}

#[test]
fn announce_month_effect_skipped_on_boundary_failure() {
    // Codex N17 (P2): when month_step_plan can't shift focused_date (e.g.,
    // calendar boundary), it must return None so AnnounceMonth doesn't
    // fire a spurious live-region announcement.
    //
    // We can't deterministically hit a Gregorian calendar boundary in a
    // portable test, but we can pin the happy-path invariant that
    // AnnounceMonth fires when the shift succeeds.
    let mut svc = service();
    let result = svc.send(Event::NextMonth);
    assert!(
        result
            .pending_effects
            .iter()
            .any(|e| e.name == Effect::AnnounceMonth),
        "happy-path NextMonth must still fire AnnounceMonth",
    );
    assert!(result.context_changed);
}

// ────────────────────────────────────────────────────────────────────
// Codex review regressions, pass 6 (PR #688)
// ────────────────────────────────────────────────────────────────────

#[test]
fn unavailable_suffix_label_uses_messages_default() {
    // Codex N20 (P2): the "(unavailable)" suffix in the cell aria-label
    // must come from `Messages` so it can be localized. The default value
    // matches what was previously hard-coded, so existing snapshots stay
    // the same.
    let predicate = Callback::new_ref(|d: &CalendarDate| d.day() == 12);
    let svc = service_with(props().is_date_unavailable(Some(predicate)), en_us());
    let api = svc.connect(&|_| {});
    let trigger = api.cell_trigger_attrs(&date(2024, 1, 12));
    let label = attr(&trigger, HtmlAttr::Aria(AriaAttr::Label)).unwrap_or_default();
    assert!(
        label.ends_with("(unavailable)"),
        "default Messages must keep the existing English suffix; got {label:?}",
    );
}

#[test]
fn unavailable_suffix_label_routes_through_messages_override() {
    // Localised Messages must reach the aria-label so a German consumer
    // can replace `(unavailable)` with `(nicht verfügbar)`.
    let predicate = Callback::new_ref(|d: &CalendarDate| d.day() == 12);
    let messages = Messages {
        unavailable_suffix: MessageFn::static_str("(nicht verfügbar)"),
        ..Messages::default()
    };
    let svc = Service::<Machine>::new(
        props().is_date_unavailable(Some(predicate)),
        &env(en_us()),
        &messages,
    );
    let api = svc.connect(&|_| {});
    let trigger = api.cell_trigger_attrs(&date(2024, 1, 12));
    let label = attr(&trigger, HtmlAttr::Aria(AriaAttr::Label)).unwrap_or_default();
    assert!(
        label.ends_with("(nicht verfügbar)"),
        "localized suffix must flow through; got {label:?}",
    );
}

#[test]
fn disabled_suffix_label_routes_through_messages_override() {
    let messages = Messages {
        disabled_suffix: MessageFn::static_str("(deaktiviert)"),
        ..Messages::default()
    };
    let svc = Service::<Machine>::new(
        props().min(Some(date(2024, 1, 20))),
        &env(en_us()),
        &messages,
    );
    let api = svc.connect(&|_| {});
    let trigger = api.cell_trigger_attrs(&date(2024, 1, 5));
    let label = attr(&trigger, HtmlAttr::Aria(AriaAttr::Label)).unwrap_or_default();
    assert!(
        label.ends_with("(deaktiviert)"),
        "localized disabled suffix must flow through; got {label:?}",
    );
}
