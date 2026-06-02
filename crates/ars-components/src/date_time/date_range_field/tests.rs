//! Unit and snapshot tests for the `DateRangeField` component.
//!
//! Test names that begin with `snapshot_` use `insta::assert_snapshot!` and
//! commit golden output under `snapshots/`. Every other test is a pure
//! state-machine or connect-API assertion that does not depend on `.snap`
//! files.

use alloc::{format, string::String, sync::Arc, vec::Vec};
use core::{cell::RefCell, cmp::Ordering};

use ars_core::{AriaAttr, AttrMap, ComponentPart, ConnectApi, Env, HtmlAttr, Service};
use ars_i18n::{CalendarDate, DateRange, Locale, StubIntlBackend, locales::en_us};
use insta::assert_snapshot;

use super::{super::date_field, *};

// ────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────

fn date(year: i32, month: u8, day: u8) -> CalendarDate {
    CalendarDate::new_gregorian(year, month, day).expect("valid test date")
}

fn range(start: CalendarDate, end: CalendarDate) -> DateRange {
    DateRange::new(start, end).expect("ordered test range")
}

fn props() -> Props {
    Props {
        id: String::from("trip"),
        ..Props::default()
    }
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

/// Drives a child `DateField` from the given props and returns its field-group
/// attributes, used to assert the start/end sub-group accessibility.
fn child_field_group_attrs(child: date_field::Props) -> AttrMap {
    let service =
        Service::<date_field::Machine>::new(child, &env(en_us()), &date_field::Messages::default());

    service.connect(&|_| {}).field_group_attrs()
}

// ────────────────────────────────────────────────────────────────────
// Initial state
// ────────────────────────────────────────────────────────────────────

#[test]
fn initial_state_is_idle() {
    let svc = service();

    assert_eq!(*svc.state(), State::Idle);
    assert!(svc.context().active_field.is_none());
}

#[test]
fn initial_value_defaults_to_none() {
    let svc = service();

    assert_eq!(*svc.context().value.get(), None);
}

#[test]
fn default_value_seeds_range() {
    let initial = range(date(2025, 6, 1), date(2025, 6, 15));

    let svc = service_with(
        Props {
            default_value: Some(initial.clone()),
            ..props()
        },
        en_us(),
    );

    assert_eq!(*svc.context().value.get(), Some(initial));
}

#[test]
fn controlled_value_overrides_default() {
    let controlled = range(date(2025, 1, 1), date(2025, 1, 31));

    let svc = service_with(
        Props {
            value: Some(Some(controlled.clone())),
            default_value: Some(range(date(2030, 1, 1), date(2030, 1, 2))),
            ..props()
        },
        en_us(),
    );

    assert_eq!(*svc.context().value.get(), Some(controlled));
}

// ────────────────────────────────────────────────────────────────────
// Focus tracking
// ────────────────────────────────────────────────────────────────────

#[test]
fn focus_start_enters_start_focused() {
    let mut svc = service();

    drop(svc.send(Event::FocusStart));

    assert_eq!(*svc.state(), State::StartFocused);
    assert_eq!(svc.context().active_field, Some(ActiveField::Start));
}

#[test]
fn focus_end_enters_end_focused() {
    let mut svc = service();

    drop(svc.send(Event::FocusEnd));

    assert_eq!(*svc.state(), State::EndFocused);
    assert_eq!(svc.context().active_field, Some(ActiveField::End));
}

#[test]
fn tab_from_start_to_end_then_blur() {
    let mut svc = service();

    // Tab navigates from the last start segment into the first end segment;
    // the agnostic core models this as the focus moving between fields.
    drop(svc.send(Event::FocusStart));

    assert_eq!(*svc.state(), State::StartFocused);

    drop(svc.send(Event::FocusEnd));

    assert_eq!(*svc.state(), State::EndFocused);

    drop(svc.send(Event::BlurAll));

    assert_eq!(*svc.state(), State::Idle);
    assert!(svc.context().active_field.is_none());
}

// ────────────────────────────────────────────────────────────────────
// Disabled / readonly guards
// ────────────────────────────────────────────────────────────────────

#[test]
fn disabled_ignores_all_events() {
    let mut svc = service_with(
        Props {
            disabled: true,
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::FocusStart));
    drop(svc.send(Event::SetRange(Some(range(
        date(2025, 6, 1),
        date(2025, 6, 15),
    )))));

    assert_eq!(*svc.state(), State::Idle);
    assert_eq!(*svc.context().value.get(), None);
}

#[test]
fn readonly_ignores_value_changes_but_allows_focus() {
    let mut svc = service_with(
        Props {
            readonly: true,
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::SetRange(Some(range(
        date(2025, 6, 1),
        date(2025, 6, 15),
    )))));

    assert_eq!(*svc.context().value.get(), None);

    drop(svc.send(Event::StartValueChange(Some(date(2025, 6, 1)))));

    assert_eq!(svc.context().start_date, None);

    drop(svc.send(Event::EndValueChange(Some(date(2025, 6, 15)))));

    assert_eq!(svc.context().end_date, None);

    drop(svc.send(Event::FocusStart));

    assert_eq!(*svc.state(), State::StartFocused);
}

// ────────────────────────────────────────────────────────────────────
// Range coordination and normalization
// ────────────────────────────────────────────────────────────────────

#[test]
fn set_range_updates_value() {
    let mut svc = service();

    let selected = range(date(2025, 6, 1), date(2025, 6, 30));

    let result = svc.send(Event::SetRange(Some(selected.clone())));

    // Value changes are context-only — no state transition, only a context mutation.
    assert!(!result.state_changed);
    assert!(result.context_changed);

    assert_eq!(*svc.context().value.get(), Some(selected));
}

#[test]
fn start_then_end_completes_range() {
    let mut svc = service();

    // Setting only the start does not yet produce a complete range.
    drop(svc.send(Event::StartValueChange(Some(date(2025, 6, 1)))));

    assert_eq!(*svc.context().value.get(), None);

    drop(svc.send(Event::EndValueChange(Some(date(2025, 6, 15)))));

    assert_eq!(
        *svc.context().value.get(),
        Some(range(date(2025, 6, 1), date(2025, 6, 15)))
    );
}

#[test]
fn start_change_after_end_normalizes_when_out_of_order() {
    let mut svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 10), date(2025, 6, 20))),
            ..props()
        },
        en_us(),
    );

    // Move the start to *after* the existing end; the range must normalize so
    // start never exceeds end.
    drop(svc.send(Event::StartValueChange(Some(date(2025, 6, 25)))));

    let stored = svc.context().value.get().clone().expect("range present");

    assert_eq!(stored.start, date(2025, 6, 20));
    assert_eq!(stored.end, date(2025, 6, 25));
    assert!(matches!(
        stored.start.compare_within_calendar(&stored.end),
        Some(Ordering::Less | Ordering::Equal)
    ));
}

#[test]
fn end_change_before_start_normalizes() {
    let mut svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 10), date(2025, 6, 20))),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::EndValueChange(Some(date(2025, 6, 1)))));

    let stored = svc.context().value.get().clone().expect("range present");

    assert_eq!(stored.start, date(2025, 6, 1));
    assert_eq!(stored.end, date(2025, 6, 10));
}

#[test]
fn clearing_start_clears_range() {
    let mut svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::StartValueChange(None)));

    assert_eq!(*svc.context().value.get(), None);
}

// ────────────────────────────────────────────────────────────────────
// Validation
// ────────────────────────────────────────────────────────────────────

#[test]
fn range_within_bounds_is_valid() {
    let svc = service_with(
        Props {
            min: Some(date(2025, 1, 1)),
            max: Some(date(2025, 12, 31)),
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    assert!(!svc.context().is_invalid());

    let api = svc.connect(&|_| {});

    assert!(!api.is_invalid());
    assert!(attr(&api.root_attrs(), HtmlAttr::Data("ars-invalid")).is_none());
}

#[test]
fn range_below_min_is_invalid() {
    let svc = service_with(
        Props {
            min: Some(date(2025, 6, 1)),
            default_value: Some(range(date(2025, 5, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    assert!(svc.context().is_invalid());

    let api = svc.connect(&|_| {});

    assert!(api.is_invalid());
    assert_eq!(
        attr(&api.root_attrs(), HtmlAttr::Data("ars-invalid")).as_deref(),
        Some("true")
    );
}

#[test]
fn range_above_max_is_invalid() {
    let svc = service_with(
        Props {
            max: Some(date(2025, 6, 30)),
            default_value: Some(range(date(2025, 6, 1), date(2025, 7, 15))),
            ..props()
        },
        en_us(),
    );

    assert!(svc.context().is_invalid());
}

#[test]
fn empty_range_is_never_invalid() {
    let svc = service_with(
        Props {
            min: Some(date(2025, 6, 1)),
            max: Some(date(2025, 6, 30)),
            ..props()
        },
        en_us(),
    );

    assert!(!svc.context().is_invalid());
}

// ────────────────────────────────────────────────────────────────────
// Connect API — root / group
// ────────────────────────────────────────────────────────────────────

#[test]
fn root_is_a_group_labelled_by_label() {
    let svc = service();

    let api = svc.connect(&|_| {});

    let attrs = api.root_attrs();

    assert_eq!(attr(&attrs, HtmlAttr::Role).as_deref(), Some("group"));
    assert_eq!(
        attr(&attrs, HtmlAttr::Aria(AriaAttr::LabelledBy)).as_deref(),
        Some("trip-label")
    );
    assert_eq!(
        attr(&attrs, HtmlAttr::Data("ars-state")).as_deref(),
        Some("idle")
    );
}

#[test]
fn required_sets_aria_required() {
    let svc = service_with(
        Props {
            required: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.root_attrs(), HtmlAttr::Aria(AriaAttr::Required)).as_deref(),
        Some("true")
    );
}

#[test]
fn describedby_references_present_description_and_error() {
    let svc = service_with(
        Props {
            has_description: true,
            has_error_message: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.root_attrs(), HtmlAttr::Aria(AriaAttr::DescribedBy)).as_deref(),
        Some("trip-description trip-error-message")
    );
}

#[test]
fn two_subgroups_are_groups_with_distinct_labels() {
    let svc = service();

    let api = svc.connect(&|_| {});

    let start_group = child_field_group_attrs(api.start_field_props());
    let end_group = child_field_group_attrs(api.end_field_props());

    assert_eq!(attr(&start_group, HtmlAttr::Role).as_deref(), Some("group"));
    assert_eq!(
        attr(&start_group, HtmlAttr::Aria(AriaAttr::Label)).as_deref(),
        Some("Start date")
    );
    assert_eq!(attr(&end_group, HtmlAttr::Role).as_deref(), Some("group"));
    assert_eq!(
        attr(&end_group, HtmlAttr::Aria(AriaAttr::Label)).as_deref(),
        Some("End date")
    );
}

// ────────────────────────────────────────────────────────────────────
// Connect API — separator
// ────────────────────────────────────────────────────────────────────

#[test]
fn separator_is_aria_hidden_with_text() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.separator_attrs(), HtmlAttr::Aria(AriaAttr::Hidden)).as_deref(),
        Some("true")
    );
    assert_eq!(api.separator_text(), " \u{2013} ");
}

// ────────────────────────────────────────────────────────────────────
// Connect API — child field props (min/max coordination)
// ────────────────────────────────────────────────────────────────────

#[test]
fn child_field_props_carry_distinct_ids_and_force_leading_zeros() {
    let svc = service_with(
        Props {
            force_leading_zeros: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let start = api.start_field_props();
    let end = api.end_field_props();

    assert_eq!(start.id, "trip-start");
    assert_eq!(end.id, "trip-end");
    assert!(start.force_leading_zeros);
    assert!(end.force_leading_zeros);
}

#[test]
fn global_min_max_apply_to_both_fields_when_no_range() {
    let svc = service_with(
        Props {
            min: Some(date(2025, 1, 1)),
            max: Some(date(2025, 12, 31)),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let start = api.start_field_props();
    let end = api.end_field_props();

    assert_eq!(start.min_value, Some(date(2025, 1, 1)));
    assert_eq!(start.max_value, Some(date(2025, 12, 31)));
    assert_eq!(end.min_value, Some(date(2025, 1, 1)));
    assert_eq!(end.max_value, Some(date(2025, 12, 31)));
}

#[test]
fn selected_range_constrains_cross_field_bounds() {
    let svc = service_with(
        Props {
            min: Some(date(2025, 1, 1)),
            max: Some(date(2025, 12, 31)),
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 30))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let start = api.start_field_props();
    let end = api.end_field_props();

    // Start cannot exceed the current end; end cannot precede the current start.
    assert_eq!(start.min_value, Some(date(2025, 1, 1)));
    assert_eq!(start.max_value, Some(date(2025, 6, 30)));
    assert_eq!(end.min_value, Some(date(2025, 6, 1)));
    assert_eq!(end.max_value, Some(date(2025, 12, 31)));
    assert_eq!(start.value, Some(Some(date(2025, 6, 1))));
    assert_eq!(end.value, Some(Some(date(2025, 6, 30))));
}

#[test]
fn child_fields_inherit_disabled_readonly_required() {
    let svc = service_with(
        Props {
            disabled: true,
            readonly: true,
            required: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let start = api.start_field_props();

    assert!(start.disabled);
    assert!(start.readonly);
    assert!(start.required);
}

// ────────────────────────────────────────────────────────────────────
// Connect API — form integration
// ────────────────────────────────────────────────────────────────────

#[test]
fn hidden_input_carries_combined_iso_interval() {
    let svc = service_with(
        Props {
            name: Some(String::from("trip-range")),
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let attrs = api.hidden_input_attrs();

    assert_eq!(attr(&attrs, HtmlAttr::Type).as_deref(), Some("hidden"));
    assert_eq!(attr(&attrs, HtmlAttr::Name).as_deref(), Some("trip-range"));
    assert_eq!(
        attr(&attrs, HtmlAttr::Value).as_deref(),
        Some("2025-06-01/2025-06-15")
    );
}

#[test]
fn hidden_input_is_empty_without_range() {
    let svc = service_with(
        Props {
            name: Some(String::from("trip-range")),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.hidden_input_attrs(), HtmlAttr::Value).as_deref(),
        Some("")
    );
}

#[test]
fn separate_start_and_end_hidden_inputs() {
    let svc = service_with(
        Props {
            start_name: Some(String::from("check-in")),
            end_name: Some(String::from("check-out")),
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let start = api.start_hidden_input_attrs();
    let end = api.end_hidden_input_attrs();

    assert_eq!(attr(&start, HtmlAttr::Name).as_deref(), Some("check-in"));
    assert_eq!(attr(&start, HtmlAttr::Value).as_deref(), Some("2025-06-01"));
    assert_eq!(attr(&end, HtmlAttr::Name).as_deref(), Some("check-out"));
    assert_eq!(attr(&end, HtmlAttr::Value).as_deref(), Some("2025-06-15"));
}

#[test]
fn separate_hidden_inputs_are_empty_without_range() {
    let svc = service_with(
        Props {
            start_name: Some(String::from("check-in")),
            end_name: Some(String::from("check-out")),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.start_hidden_input_attrs(), HtmlAttr::Value).as_deref(),
        Some("")
    );
    assert_eq!(
        attr(&api.end_hidden_input_attrs(), HtmlAttr::Value).as_deref(),
        Some("")
    );
}

// ────────────────────────────────────────────────────────────────────
// Connect API — range description and convenience getters
// ────────────────────────────────────────────────────────────────────

#[test]
fn range_description_is_none_when_empty() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert!(api.range_description().is_none());
}

#[test]
fn range_description_describes_selected_range() {
    let svc = service_with(
        Props {
            default_value: Some(range(date(2025, 3, 1), date(2025, 3, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        api.range_description().as_deref(),
        Some("March 1, 2025 to March 15, 2025")
    );
}

#[test]
fn convenience_getters_reflect_state() {
    let mut svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::FocusEnd));

    let api = svc.connect(&|_| {});

    assert!(api.is_focused());
    assert_eq!(api.active_field(), Some(ActiveField::End));
    assert_eq!(
        api.selected_range(),
        Some(&range(date(2025, 6, 1), date(2025, 6, 15)))
    );
    assert_eq!(api.state_name(), "end-focused");
}

// ────────────────────────────────────────────────────────────────────
// Connect API — event dispatch
// ────────────────────────────────────────────────────────────────────

#[test]
fn dispatch_methods_send_expected_events() {
    let recorded: RefCell<Vec<Event>> = RefCell::new(Vec::new());
    let send = |event: Event| recorded.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&send);

    api.focus_start();
    api.focus_end();
    api.blur();
    api.set_range(None);
    api.set_start_value(Some(date(2025, 6, 1)));
    api.set_end_value(None);

    assert_eq!(
        *recorded.borrow(),
        Vec::from([
            Event::FocusStart,
            Event::FocusEnd,
            Event::BlurAll,
            Event::SetRange(None),
            Event::StartValueChange(Some(date(2025, 6, 1))),
            Event::EndValueChange(None),
        ])
    );
}

#[test]
fn context_and_api_debug_redact_intl_backend() {
    let svc = service_with(
        Props {
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    let ctx_debug = format!("{:?}", svc.context());

    assert!(ctx_debug.contains("Context"));
    assert!(ctx_debug.contains("start_date"));
    // The `dyn IntlBackend` is redacted rather than formatted.
    assert!(ctx_debug.contains("<dyn IntlBackend>"));

    let api = svc.connect(&|_| {});

    let api_debug = format!("{api:?}");

    assert!(api_debug.contains("Api"));
}

// ────────────────────────────────────────────────────────────────────
// Anatomy / Part coverage
// ────────────────────────────────────────────────────────────────────

#[test]
fn part_attrs_cover_every_part() {
    let svc = service();

    let api = svc.connect(&|_| {});

    for part in Part::all() {
        let name = part.name();

        let attrs = api.part_attrs(part);

        assert_eq!(
            attr(&attrs, HtmlAttr::Data("ars-part")).as_deref(),
            Some(name),
            "part {name} must carry its data-ars-part marker"
        );
        assert_eq!(
            attr(&attrs, HtmlAttr::Data("ars-scope")).as_deref(),
            Some("date-range-field")
        );
    }
}

#[test]
fn props_builder_sets_every_field() {
    let built = Props::new()
        .id("trip")
        .value(Some(range(date(2025, 6, 1), date(2025, 6, 15))))
        .default_value(Some(range(date(2025, 1, 1), date(2025, 1, 2))))
        .min(Some(date(2024, 1, 1)))
        .max(Some(date(2026, 1, 1)))
        .disabled(true)
        .readonly(true)
        .required(true)
        .name(Some(String::from("range")))
        .start_name(Some(String::from("start")))
        .end_name(Some(String::from("end")))
        .force_leading_zeros(true)
        .has_description(true)
        .has_error_message(true);

    assert_eq!(built.id, "trip");
    assert_eq!(
        built.value,
        Some(Some(range(date(2025, 6, 1), date(2025, 6, 15))))
    );
    assert_eq!(
        built.default_value,
        Some(range(date(2025, 1, 1), date(2025, 1, 2)))
    );
    assert_eq!(built.min, Some(date(2024, 1, 1)));
    assert_eq!(built.max, Some(date(2026, 1, 1)));
    assert!(built.disabled);
    assert!(built.readonly);
    assert!(built.required);
    assert_eq!(built.name.as_deref(), Some("range"));
    assert_eq!(built.start_name.as_deref(), Some("start"));
    assert_eq!(built.end_name.as_deref(), Some("end"));
    assert!(built.force_leading_zeros);
    assert!(built.has_description);
    assert!(built.has_error_message);
}

// ────────────────────────────────────────────────────────────────────
// Snapshots
// ────────────────────────────────────────────────────────────────────

#[test]
fn snapshot_root_idle() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_start_focused() {
    let mut svc = service();

    drop(svc.send(Event::FocusStart));

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_end_focused() {
    let mut svc = service();

    drop(svc.send(Event::FocusEnd));

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_disabled() {
    let svc = service_with(
        Props {
            disabled: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_readonly() {
    let svc = service_with(
        Props {
            readonly: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_required() {
    let svc = service_with(
        Props {
            required: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_invalid() {
    let svc = service_with(
        Props {
            max: Some(date(2025, 6, 30)),
            default_value: Some(range(date(2025, 6, 1), date(2025, 7, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_with_description_and_error() {
    let svc = service_with(
        Props {
            has_description: true,
            has_error_message: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_label() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.label_attrs()));
}

#[test]
fn snapshot_start_field_marker() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.start_field_attrs()));
}

#[test]
fn snapshot_end_field_marker() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.end_field_attrs()));
}

#[test]
fn snapshot_separator() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.separator_attrs()));
}

#[test]
fn snapshot_description() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.description_attrs()));
}

#[test]
fn snapshot_error_message() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.error_message_attrs()));
}

#[test]
fn snapshot_hidden_input_empty() {
    let svc = service_with(
        Props {
            name: Some(String::from("trip-range")),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.hidden_input_attrs()));
}

#[test]
fn snapshot_hidden_input_with_range() {
    let svc = service_with(
        Props {
            name: Some(String::from("trip-range")),
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.hidden_input_attrs()));
}

#[test]
fn snapshot_start_hidden_input() {
    let svc = service_with(
        Props {
            start_name: Some(String::from("check-in")),
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.start_hidden_input_attrs()));
}

#[test]
fn snapshot_end_hidden_input() {
    let svc = service_with(
        Props {
            end_name: Some(String::from("check-out")),
            default_value: Some(range(date(2025, 6, 1), date(2025, 6, 15))),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(snapshot_attrs(&api.end_hidden_input_attrs()));
}
