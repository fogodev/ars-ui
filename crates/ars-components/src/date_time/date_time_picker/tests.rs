//! Unit and snapshot tests for the `DateTimePicker` component.
//!
//! Test names that begin with `snapshot_` use `insta::assert_snapshot!` and
//! commit golden output under `snapshots/`. Every other test is a pure
//! state-machine or connect-API assertion that does not depend on `.snap` files.

use alloc::{format, string::String, sync::Arc, vec, vec::Vec};
use core::cell::RefCell;

use ars_core::{AriaAttr, AttrMap, ComponentPart, Direction, Env, HtmlAttr, SendResult, Service};
use ars_i18n::{
    CalendarDate, CalendarDateFields, CalendarDateTime, CalendarSystem, HourCycle, Locale,
    StubIntlBackend, Time,
    locales::{de_de, en_us},
};
use ars_interactions::{KeyboardEventData, KeyboardKey};
use insta::assert_snapshot;

use super::*;
use crate::date_time::time_field::TimeGranularity;

// ────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────

fn date(year: i32, month: u8, day: u8) -> CalendarDate {
    CalendarDate::new_gregorian(year, month, day).expect("valid test date")
}

fn time(hour: u8, minute: u8, second: u8) -> Time {
    Time::new(hour, minute, second, 0).expect("valid test time")
}

fn datetime(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> CalendarDateTime {
    CalendarDateTime::new(date(year, month, day), time(hour, minute, second))
}

fn props() -> Props {
    Props {
        id: String::from("date-time-picker"),
        label: String::from("Appointment"),
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

fn effects(result: SendResult<Machine>) -> Vec<Effect> {
    result
        .pending_effects
        .into_iter()
        .map(|effect| effect.name)
        .collect()
}

fn keyboard(key: KeyboardKey) -> KeyboardEventData {
    KeyboardEventData {
        key,
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

fn segment_kinds(segments: &[DateSegment]) -> Vec<DateSegmentKind> {
    segments
        .iter()
        .filter(|segment| segment.is_editable)
        .map(|segment| segment.kind)
        .collect()
}

// ────────────────────────────────────────────────────────────────────
// Initial state
// ────────────────────────────────────────────────────────────────────

#[test]
fn init_is_idle_and_closed() {
    let svc = service();

    assert_eq!(*svc.state(), State::Idle);
    assert!(!svc.context().open);
    assert_eq!(*svc.context().value.get(), None);
}

#[test]
fn en_us_resolves_h12_with_day_period() {
    let svc = service();

    assert_eq!(svc.context().hour_cycle, HourCycle::H12);
    assert!(
        svc.context()
            .time_segments
            .iter()
            .any(|segment| segment.kind == DateSegmentKind::DayPeriod)
    );
}

#[test]
fn de_de_resolves_h23_without_day_period() {
    let svc = service_with(props(), de_de());

    assert_eq!(svc.context().hour_cycle, HourCycle::H23);
    assert!(
        !svc.context()
            .time_segments
            .iter()
            .any(|segment| segment.kind == DateSegmentKind::DayPeriod)
    );
}

// ────────────────────────────────────────────────────────────────────
// Issue acceptance tests
// ────────────────────────────────────────────────────────────────────

#[test]
fn date_and_time_segments_both_present() {
    let svc = service();

    let api = svc.connect(&|_| {});

    let date_segments = segment_kinds(api.date_segments());
    let time_segments = segment_kinds(api.time_segments());

    assert!(date_segments.contains(&DateSegmentKind::Year));
    assert!(date_segments.contains(&DateSegmentKind::Month));
    assert!(date_segments.contains(&DateSegmentKind::Day));
    assert!(time_segments.contains(&DateSegmentKind::Hour));
    assert!(time_segments.contains(&DateSegmentKind::Minute));

    // The calendar popover surface (trigger + dialog content) is present.
    assert_eq!(
        attr(&api.content_attrs(), HtmlAttr::Role).as_deref(),
        Some("dialog")
    );
    assert!(attr(&api.trigger_attrs(), HtmlAttr::Aria(AriaAttr::Controls)).is_some());
}

#[test]
fn combined_value_renders_iso_datetime() {
    let svc = service_with(
        Props {
            default_value: Some(datetime(2024, 3, 15, 14, 30, 0)),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.hidden_input_attrs(), HtmlAttr::Value).as_deref(),
        Some("2024-03-15T14:30:00")
    );
}

#[test]
fn selecting_date_preserves_time() {
    let mut svc = service_with(
        Props {
            default_value: Some(datetime(2024, 3, 15, 14, 30, 0)),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::CalendarSelectDate(date(2025, 1, 2))));

    assert_eq!(svc.context().date_value, Some(date(2025, 1, 2)));
    assert_eq!(svc.context().time_value, Some(time(14, 30, 0)));
    assert_eq!(
        *svc.context().value.get(),
        Some(datetime(2025, 1, 2, 14, 30, 0))
    );
}

#[test]
fn editing_time_preserves_date() {
    let mut svc = service_with(
        Props {
            default_value: Some(datetime(2024, 3, 15, 14, 30, 0)),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::SegmentChange {
        segment: DateSegmentKind::Minute,
        value: 45,
    }));

    assert_eq!(svc.context().date_value, Some(date(2024, 3, 15)));
    assert_eq!(svc.context().time_value, Some(time(14, 45, 0)));
    assert_eq!(
        *svc.context().value.get(),
        Some(datetime(2024, 3, 15, 14, 45, 0))
    );
}

#[test]
fn form_integration_hidden_input_carries_name_and_iso_value() {
    let svc = service_with(
        Props {
            name: Some(String::from("appointment")),
            default_value: Some(datetime(2024, 3, 15, 9, 5, 0)),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let attrs = api.hidden_input_attrs();

    assert_eq!(attr(&attrs, HtmlAttr::Type).as_deref(), Some("hidden"));
    assert_eq!(attr(&attrs, HtmlAttr::Name).as_deref(), Some("appointment"));
    assert_eq!(
        attr(&attrs, HtmlAttr::Value).as_deref(),
        Some("2024-03-15T09:05:00")
    );
}

#[test]
fn value_commit_clamps_below_minimum_across_date_and_time() {
    let mut svc = service_with(
        Props {
            min_value: Some(datetime(2024, 6, 1, 9, 0, 0)),
            max_value: Some(datetime(2024, 12, 31, 17, 0, 0)),
            ..props()
        },
        en_us(),
    );

    // Earlier date than the minimum: clamps the whole datetime up to the min.
    drop(svc.send(Event::ValueCommit(Some(datetime(2024, 5, 1, 12, 0, 0)))));

    assert_eq!(
        *svc.context().value.get(),
        Some(datetime(2024, 6, 1, 9, 0, 0))
    );
}

#[test]
fn value_commit_clamps_above_maximum_across_date_and_time() {
    let mut svc = service_with(
        Props {
            min_value: Some(datetime(2024, 6, 1, 9, 0, 0)),
            max_value: Some(datetime(2024, 12, 31, 17, 0, 0)),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::ValueCommit(Some(datetime(2025, 1, 1, 12, 0, 0)))));

    assert_eq!(
        *svc.context().value.get(),
        Some(datetime(2024, 12, 31, 17, 0, 0))
    );
}

#[test]
fn same_date_time_only_clamp() {
    let mut svc = service_with(
        Props {
            min_value: Some(datetime(2024, 6, 1, 13, 0, 0)),
            ..props()
        },
        en_us(),
    );

    // Same date, earlier time than min: clamps to the minimum time.
    drop(svc.send(Event::ValueCommit(Some(datetime(2024, 6, 1, 12, 0, 0)))));

    assert_eq!(
        *svc.context().value.get(),
        Some(datetime(2024, 6, 1, 13, 0, 0))
    );
}

#[test]
fn connect_api_groups_date_and_time_with_labels() {
    let svc = service();

    let api = svc.connect(&|_| {});

    let control = api.control_attrs();

    assert_eq!(attr(&control, HtmlAttr::Role).as_deref(), Some("group"));
    assert_eq!(
        attr(&control, HtmlAttr::Aria(AriaAttr::LabelledBy)).as_deref(),
        Some("date-time-picker-label")
    );

    let date_group = api.date_segment_group_attrs();

    assert_eq!(attr(&date_group, HtmlAttr::Role).as_deref(), Some("group"));
    assert_eq!(
        attr(&date_group, HtmlAttr::Aria(AriaAttr::Label)).as_deref(),
        Some("Date")
    );

    let time_group = api.time_segment_group_attrs();

    assert_eq!(attr(&time_group, HtmlAttr::Role).as_deref(), Some("group"));
    assert_eq!(
        attr(&time_group, HtmlAttr::Aria(AriaAttr::Label)).as_deref(),
        Some("Time")
    );
}

// ────────────────────────────────────────────────────────────────────
// Popover open / close
// ────────────────────────────────────────────────────────────────────

#[test]
fn open_transitions_to_open_and_focuses_calendar() {
    let mut svc = service();

    assert_eq!(effects(svc.send(Event::Open)), vec![Effect::FocusCalendar]);
    assert_eq!(*svc.state(), State::Open);
    assert!(svc.context().open);
}

#[test]
fn open_is_a_no_op_when_already_open() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let result = svc.send(Event::Open);

    assert!(!result.state_changed);
    assert!(effects(result).is_empty());
}

#[test]
fn close_restores_focus_to_trigger() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    assert_eq!(
        effects(svc.send(Event::Close)),
        vec![Effect::RestoreFocusToTrigger]
    );
    assert_eq!(*svc.state(), State::Focused);
    assert!(!svc.context().open);
}

#[test]
fn toggle_opens_then_closes() {
    let mut svc = service();

    drop(svc.send(Event::Toggle));

    assert_eq!(*svc.state(), State::Open);

    drop(svc.send(Event::Toggle));

    assert_eq!(*svc.state(), State::Focused);
}

#[test]
fn readonly_blocks_opening() {
    let mut svc = service_with(props().readonly(true), en_us());

    let result = svc.send(Event::Open);

    assert!(!result.state_changed);
    assert_eq!(*svc.state(), State::Idle);
}

#[test]
fn disabled_blocks_all_interaction() {
    let mut svc = service_with(props().disabled(true), en_us());

    let result = svc.send(Event::Open);

    assert!(!result.state_changed);
    assert_eq!(*svc.state(), State::Idle);
}

#[test]
fn escape_closes_open_popover() {
    let mut svc = service();

    drop(svc.send(Event::Open));
    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::Escape,
    }));

    assert_eq!(*svc.state(), State::Focused);
}

#[test]
fn arrow_down_opens_when_closed() {
    let mut svc = service();

    drop(svc.send(Event::KeyDown {
        key: KeyboardKey::ArrowDown,
    }));

    assert_eq!(*svc.state(), State::Open);
}

#[test]
fn calendar_select_closes_and_focuses_first_time_segment() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let result = svc.send(Event::CalendarSelectDate(date(2024, 3, 15)));

    assert_eq!(effects(result), vec![Effect::FocusFirstTimeSegment]);
    assert_eq!(*svc.state(), State::Focused);
    assert!(!svc.context().open);
    assert_eq!(svc.context().date_value, Some(date(2024, 3, 15)));
}

// ────────────────────────────────────────────────────────────────────
// Segment focus navigation
// ────────────────────────────────────────────────────────────────────

#[test]
fn focus_next_crosses_date_to_time_boundary() {
    let mut svc = service();

    // en-US order is Month, Day, Year; the last date segment is Year.
    drop(svc.send(Event::FocusSegment(DateSegmentKind::Year)));
    drop(svc.send(Event::FocusNextSegment));

    assert_eq!(svc.context().focused_segment, Some(DateSegmentKind::Hour));
}

#[test]
fn focus_prev_crosses_time_to_date_boundary() {
    let mut svc = service();

    drop(svc.send(Event::FocusSegment(DateSegmentKind::Hour)));
    drop(svc.send(Event::FocusPrevSegment));

    assert_eq!(svc.context().focused_segment, Some(DateSegmentKind::Year));
}

#[test]
fn focus_next_from_idle_focuses_first_editable() {
    let mut svc = service();

    drop(svc.send(Event::FocusNextSegment));

    assert_eq!(svc.context().focused_segment, Some(DateSegmentKind::Month));
    assert_eq!(*svc.state(), State::Focused);
}

// ────────────────────────────────────────────────────────────────────
// Segment value editing
// ────────────────────────────────────────────────────────────────────

#[test]
fn increment_wraps_within_segment_range() {
    let mut svc = service();

    // Month max is 12; from empty, increment lands on the minimum (1).
    drop(svc.send(Event::IncrementSegment {
        segment: DateSegmentKind::Month,
    }));

    assert_eq!(svc.context().segment_value(DateSegmentKind::Month), Some(1));
}

#[test]
fn decrement_from_empty_lands_on_maximum() {
    let mut svc = service();

    drop(svc.send(Event::DecrementSegment {
        segment: DateSegmentKind::Month,
    }));

    assert_eq!(
        svc.context().segment_value(DateSegmentKind::Month),
        Some(12)
    );
}

#[test]
fn type_ahead_auto_advances_when_full() {
    let mut svc = service();

    drop(svc.send(Event::FocusSegment(DateSegmentKind::Month)));

    // Typing "1" cannot be full (could be 10..12), so it buffers and arms a timer.
    let armed = svc.send(Event::TypeIntoSegment {
        segment: DateSegmentKind::Month,
        ch: '1',
    });

    assert_eq!(effects(armed), vec![Effect::TypeBufferCommit]);

    // Typing "2" completes "12" and auto-advances to the next editable segment.
    drop(svc.send(Event::TypeIntoSegment {
        segment: DateSegmentKind::Month,
        ch: '2',
    }));

    assert_eq!(
        svc.context().segment_value(DateSegmentKind::Month),
        Some(12)
    );
    assert_eq!(svc.context().focused_segment, Some(DateSegmentKind::Day));
}

#[test]
fn type_ahead_single_digit_advances_when_unambiguous() {
    let mut svc = service();

    drop(svc.send(Event::FocusSegment(DateSegmentKind::Month)));
    // "5" * 10 = 50 > 12, so a second digit is impossible: commit and advance.
    drop(svc.send(Event::TypeIntoSegment {
        segment: DateSegmentKind::Month,
        ch: '5',
    }));

    assert_eq!(svc.context().segment_value(DateSegmentKind::Month), Some(5));
    assert_eq!(svc.context().focused_segment, Some(DateSegmentKind::Day));
}

#[test]
fn day_period_accepts_a_and_p() {
    let mut svc = service();

    drop(svc.send(Event::TypeIntoSegment {
        segment: DateSegmentKind::DayPeriod,
        ch: 'p',
    }));

    assert_eq!(
        svc.context().segment_value(DateSegmentKind::DayPeriod),
        Some(1)
    );

    drop(svc.send(Event::TypeIntoSegment {
        segment: DateSegmentKind::DayPeriod,
        ch: 'a',
    }));

    assert_eq!(
        svc.context().segment_value(DateSegmentKind::DayPeriod),
        Some(0)
    );
}

#[test]
fn day_period_rejects_other_characters() {
    let mut svc = service();

    let result = svc.send(Event::TypeIntoSegment {
        segment: DateSegmentKind::DayPeriod,
        ch: 'x',
    });

    assert!(!result.context_changed);
    assert_eq!(
        svc.context().segment_value(DateSegmentKind::DayPeriod),
        None
    );
}

#[test]
fn clear_segment_clears_value_and_resets_value() {
    let mut svc = service_with(
        Props {
            default_value: Some(datetime(2024, 3, 15, 14, 30, 0)),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::ClearSegment {
        segment: DateSegmentKind::Minute,
    }));

    assert_eq!(svc.context().segment_value(DateSegmentKind::Minute), None);
    assert_eq!(*svc.context().value.get(), None);
}

#[test]
fn clear_all_resets_every_editable_segment() {
    let mut svc = service_with(
        Props {
            default_value: Some(datetime(2024, 3, 15, 14, 30, 0)),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::ClearAll));

    assert_eq!(*svc.state(), State::Idle);
    assert_eq!(*svc.context().value.get(), None);
    assert_eq!(svc.context().date_value, None);
    assert_eq!(svc.context().time_value, None);
    assert!(
        svc.context()
            .all_segments()
            .filter(|segment| segment.is_editable)
            .all(|segment| segment.value.is_none())
    );
}

#[test]
fn readonly_blocks_segment_editing() {
    let mut svc = service_with(props().readonly(true), en_us());

    let result = svc.send(Event::IncrementSegment {
        segment: DateSegmentKind::Month,
    });

    assert!(!result.context_changed);
    assert_eq!(svc.context().segment_value(DateSegmentKind::Month), None);
}

// ────────────────────────────────────────────────────────────────────
// Focus in / out
// ────────────────────────────────────────────────────────────────────

#[test]
fn focus_in_moves_idle_to_focused() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));

    assert_eq!(*svc.state(), State::Focused);
}

#[test]
fn focus_out_commits_buffer_and_returns_to_idle() {
    let mut svc = service();

    drop(svc.send(Event::FocusSegment(DateSegmentKind::Month)));
    drop(svc.send(Event::TypeIntoSegment {
        segment: DateSegmentKind::Month,
        ch: '3',
    }));
    drop(svc.send(Event::FocusOut));

    assert_eq!(*svc.state(), State::Idle);
    assert_eq!(svc.context().focused_segment, None);
    assert_eq!(svc.context().segment_value(DateSegmentKind::Month), Some(3));
}

// ────────────────────────────────────────────────────────────────────
// Controlled prop sync
// ────────────────────────────────────────────────────────────────────

#[test]
fn sync_props_adopts_controlled_value() {
    let mut svc = service_with(
        Props {
            value: Some(None),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::SyncProps(Box::new(Props {
        value: Some(Some(datetime(2024, 3, 15, 8, 0, 0))),
        ..props()
    }))));

    assert_eq!(
        *svc.context().value.get(),
        Some(datetime(2024, 3, 15, 8, 0, 0))
    );
    assert_eq!(svc.context().date_value, Some(date(2024, 3, 15)));
}

#[test]
fn sync_props_can_lift_disabled_state() {
    let mut svc = service_with(props().disabled(true), en_us());

    // Disabled blocks normal events.
    assert!(!svc.send(Event::Open).state_changed);

    // SyncProps flows through and clears disabled.
    drop(svc.send(Event::SyncProps(Box::new(props()))));

    assert!(!svc.context().disabled);
    assert_eq!(effects(svc.send(Event::Open)), vec![Effect::FocusCalendar]);
}

// ────────────────────────────────────────────────────────────────────
// Api event handlers
// ────────────────────────────────────────────────────────────────────

#[test]
fn on_trigger_click_sends_toggle() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_trigger_click();

    assert_eq!(sent.borrow().as_slice(), &[Event::Toggle]);
}

#[test]
fn on_clear_trigger_click_sends_clear_all() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_clear_trigger_click();

    assert_eq!(sent.borrow().as_slice(), &[Event::ClearAll]);
}

#[test]
fn on_segment_keydown_arrow_up_increments() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_segment_keydown(
        DateSegmentKind::Hour,
        &keyboard(KeyboardKey::ArrowUp),
        Direction::Ltr,
    );

    assert_eq!(
        sent.borrow().as_slice(),
        &[Event::IncrementSegment {
            segment: DateSegmentKind::Hour,
        }]
    );
}

#[test]
fn on_segment_keydown_typed_digit_sends_type_into_segment() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    let mut data = keyboard(KeyboardKey::Unidentified);

    data.character = Some('5');

    api.on_segment_keydown(DateSegmentKind::Minute, &data, Direction::Ltr);

    assert_eq!(
        sent.borrow().as_slice(),
        &[Event::TypeIntoSegment {
            segment: DateSegmentKind::Minute,
            ch: '5',
        }]
    );
}

#[test]
fn on_segment_keydown_rtl_swaps_arrow_direction() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service_with(props().is_rtl(true), en_us());

    let api = svc.connect(&push);

    api.on_segment_keydown(
        DateSegmentKind::Hour,
        &keyboard(KeyboardKey::ArrowLeft),
        Direction::Rtl,
    );

    assert_eq!(sent.borrow().as_slice(), &[Event::FocusNextSegment]);
}

#[test]
fn on_content_keydown_escape_sends_close() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_content_keydown(&keyboard(KeyboardKey::Escape));

    assert_eq!(sent.borrow().as_slice(), &[Event::Close]);
}

// ────────────────────────────────────────────────────────────────────
// Calendar composition
// ────────────────────────────────────────────────────────────────────

#[test]
fn calendar_props_forward_constraints_and_today() {
    let svc = service_with(
        Props {
            default_value: Some(datetime(2024, 3, 15, 14, 30, 0)),
            min_value: Some(datetime(2024, 1, 1, 0, 0, 0)),
            max_value: Some(datetime(2024, 12, 31, 23, 0, 0)),
            today: date(2024, 3, 1),
            visible_months: 2,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    let calendar_props = api.calendar_props();

    assert_eq!(calendar_props.id, "date-time-picker-calendar");
    assert_eq!(calendar_props.value, Some(Some(date(2024, 3, 15))));
    assert_eq!(calendar_props.min, Some(date(2024, 1, 1)));
    assert_eq!(calendar_props.max, Some(date(2024, 12, 31)));
    assert_eq!(calendar_props.today, date(2024, 3, 1));
    assert_eq!(calendar_props.visible_months, 2);
}

// ────────────────────────────────────────────────────────────────────
// Conformance: anatomy + scope
// ────────────────────────────────────────────────────────────────────

#[test]
fn part_scope_is_date_time_picker() {
    let [(_, scope), _] = Part::Root.data_attrs();

    assert_eq!(scope, "date-time-picker");
}

#[test]
fn segment_attrs_expose_spinbutton_contract() {
    let svc = service();

    let api = svc.connect(&|_| {});

    let attrs = api.segment_attrs(&DateSegmentKind::Month);

    assert_eq!(attr(&attrs, HtmlAttr::Role).as_deref(), Some("spinbutton"));
    assert_eq!(
        attr(&attrs, HtmlAttr::Aria(AriaAttr::ValueMin)).as_deref(),
        Some("1")
    );
    assert_eq!(
        attr(&attrs, HtmlAttr::Aria(AriaAttr::ValueMax)).as_deref(),
        Some("12")
    );
}

#[test]
fn clear_trigger_hidden_without_value_visible_with_value() {
    let empty = service();
    let empty_api = empty.connect(&|_| {});

    assert_eq!(
        attr(&empty_api.clear_trigger_attrs(), HtmlAttr::Hidden).as_deref(),
        Some("true")
    );

    let filled = service_with(
        Props {
            default_value: Some(datetime(2024, 3, 15, 14, 30, 0)),
            ..props()
        },
        en_us(),
    );

    let filled_api = filled.connect(&|_| {});

    assert_eq!(
        attr(&filled_api.clear_trigger_attrs(), HtmlAttr::Hidden),
        None
    );
}

// ────────────────────────────────────────────────────────────────────
// Props builders + Debug/PartialEq + prop sync via the live service
// ────────────────────────────────────────────────────────────────────

#[test]
fn props_builder_chain_sets_every_field() {
    let props = Props::new()
        .id("dtp")
        .value(Some(datetime(2024, 3, 15, 14, 30, 0)))
        .default_value(Some(datetime(2024, 1, 1, 0, 0, 0)))
        .min_value(Some(datetime(2024, 1, 1, 0, 0, 0)))
        .max_value(Some(datetime(2024, 12, 31, 23, 59, 59)))
        .granularity(TimeGranularity::Second)
        .disabled(true)
        .readonly(true)
        .name(Some(String::from("appt")))
        .calendar(CalendarSystem::Gregorian)
        .hour_cycle(Some(HourCycle::H23))
        .required(true)
        .label("Appointment")
        .description(Some(String::from("help")))
        .error_message(Some(String::from("bad")))
        .invalid(true)
        .is_rtl(true)
        .visible_months(2)
        .today(date(2024, 3, 1));

    assert_eq!(props.id, "dtp");
    assert_eq!(props.value, Some(Some(datetime(2024, 3, 15, 14, 30, 0))));
    assert_eq!(props.granularity, TimeGranularity::Second);
    assert!(props.disabled && props.readonly && props.required && props.invalid && props.is_rtl);
    assert_eq!(props.hour_cycle, Some(HourCycle::H23));
    assert_eq!(props.visible_months, 2);
    assert_eq!(props.today, date(2024, 3, 1));
    assert_eq!(props.name.as_deref(), Some("appt"));
    assert_eq!(props.description.as_deref(), Some("help"));
    assert_eq!(props.error_message.as_deref(), Some("bad"));
}

#[test]
fn context_and_api_are_debug() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert!(format!("{:?}", svc.context()).contains("Context"));
    assert!(format!("{api:?}").contains("Api"));
}

#[test]
fn context_equality_compares_all_fields() {
    // Clone shares the message closures' `Arc`s, so an unmodified clone compares
    // equal; a state change then makes the live context differ.
    let mut svc = service();
    let snapshot = svc.context().clone();

    // `Close` while idle is a no-op, leaving the context untouched.
    drop(svc.send(Event::Close));
    assert_eq!(&snapshot, svc.context());

    drop(svc.send(Event::FocusSegment(DateSegmentKind::Month)));
    assert_ne!(&snapshot, svc.context());
}

#[test]
fn set_props_noop_emits_nothing_then_change_syncs_value() {
    let mut svc = service_with(
        Props {
            value: Some(None),
            ..props()
        },
        en_us(),
    );

    // Identical props → on_props_changed returns no events (Context PartialEq).
    let noop = svc.set_props(Props {
        value: Some(None),
        ..props()
    });

    assert!(!noop.state_changed);

    // Changed controlled value → SyncProps lands it through the live service.
    drop(svc.set_props(Props {
        value: Some(Some(datetime(2024, 3, 15, 8, 0, 0))),
        ..props()
    }));

    assert_eq!(
        *svc.context().value.get(),
        Some(datetime(2024, 3, 15, 8, 0, 0))
    );
}

#[test]
fn set_props_disable_while_focused_returns_to_idle() {
    let mut svc = service();

    drop(svc.send(Event::FocusSegment(DateSegmentKind::Month)));

    assert_eq!(*svc.state(), State::Focused);

    drop(svc.set_props(props().disabled(true)));

    assert_eq!(*svc.state(), State::Idle);
    assert_eq!(svc.context().focused_segment, None);
}

// ────────────────────────────────────────────────────────────────────
// Remaining transition + helper branches
// ────────────────────────────────────────────────────────────────────

#[test]
fn value_commit_none_clears_value() {
    let mut svc = service_with(
        Props {
            default_value: Some(datetime(2024, 3, 15, 14, 30, 0)),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::ValueCommit(None)));

    assert_eq!(*svc.context().value.get(), None);
    assert_eq!(svc.context().date_value, None);
    assert_eq!(svc.context().time_value, None);
}

#[test]
fn segment_change_on_date_segment_updates_date_value() {
    let mut svc = service();

    drop(svc.send(Event::SegmentChange {
        segment: DateSegmentKind::Year,
        value: 2024,
    }));
    drop(svc.send(Event::SegmentChange {
        segment: DateSegmentKind::Month,
        value: 3,
    }));
    drop(svc.send(Event::SegmentChange {
        segment: DateSegmentKind::Day,
        value: 15,
    }));

    assert_eq!(svc.context().date_value, Some(date(2024, 3, 15)));
}

#[test]
fn type_buffer_commit_lands_buffered_digits() {
    let mut svc = service();

    drop(svc.send(Event::FocusSegment(DateSegmentKind::Minute)));
    // Single "5" into minute (max 59) cannot be full, so it only buffers.
    drop(svc.send(Event::TypeIntoSegment {
        segment: DateSegmentKind::Minute,
        ch: '5',
    }));

    assert_eq!(
        svc.context().segment_value(DateSegmentKind::Minute),
        Some(5)
    );

    // The buffered "5" commits when the timer fires.
    drop(svc.send(Event::TypeBufferCommit {
        segment: DateSegmentKind::Minute,
    }));

    assert_eq!(
        svc.context().segment_value(DateSegmentKind::Minute),
        Some(5)
    );
    assert!(svc.context().type_buffer.is_empty());
}

#[test]
fn focus_next_commits_pending_buffer_before_moving() {
    let mut svc = service();

    drop(svc.send(Event::FocusSegment(DateSegmentKind::Minute)));
    drop(svc.send(Event::TypeIntoSegment {
        segment: DateSegmentKind::Minute,
        ch: '4',
    }));
    // "4" buffers (could be 40-49); FocusNext commits it then advances.
    drop(svc.send(Event::FocusNextSegment));

    assert_eq!(
        svc.context().segment_value(DateSegmentKind::Minute),
        Some(4)
    );
    assert!(svc.context().type_buffer.is_empty());
}

#[test]
fn focus_prev_from_idle_is_noop() {
    let mut svc = service();

    let result = svc.send(Event::FocusPrevSegment);

    assert!(!result.state_changed);
}

#[test]
fn second_granularity_exposes_second_segment_and_announcement() {
    let svc = service_with(
        Props {
            granularity: TimeGranularity::Second,
            default_value: Some(datetime(2024, 3, 15, 14, 30, 45)),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert!(
        api.time_segments()
            .iter()
            .any(|segment| segment.kind == DateSegmentKind::Second)
    );

    // The announcement includes seconds and the AM/PM suffix.
    let announcement = attr(&api.root_attrs(), HtmlAttr::Aria(AriaAttr::Description));

    assert_eq!(
        announcement.as_deref(),
        Some("Selected: 03/15/2024 02:30:45 PM")
    );
}

#[test]
fn h23_announcement_has_no_day_period() {
    let svc = service_with(
        Props {
            default_value: Some(datetime(2024, 3, 15, 14, 30, 0)),
            ..props()
        },
        de_de(),
    );

    let api = svc.connect(&|_| {});

    // de-DE is H23 (24-hour, DD.MM.YYYY) → no "AM/PM", "." date separator.
    let announcement = attr(&api.root_attrs(), HtmlAttr::Aria(AriaAttr::Description));

    assert_eq!(announcement.as_deref(), Some("Selected: 15.03.2024 14:30"));
}

#[test]
fn segment_attrs_marks_readonly() {
    let svc = service_with(props().readonly(true), en_us());

    let api = svc.connect(&|_| {});

    let attrs = api.segment_attrs(&DateSegmentKind::Month);

    assert_eq!(
        attr(&attrs, HtmlAttr::Aria(AriaAttr::ReadOnly)).as_deref(),
        Some("true")
    );
}

#[test]
fn segment_attrs_unknown_kind_is_empty() {
    // Era is never built into the date-time picker's segment lists, so the
    // lookup returns an empty AttrMap (no role/scope set).
    let svc = service();

    let api = svc.connect(&|_| {});

    let attrs = api.segment_attrs(&DateSegmentKind::Era);

    assert!(attr(&attrs, HtmlAttr::Role).is_none());
    assert!(attr(&attrs, HtmlAttr::Id).is_none());
}

#[test]
fn api_convenience_accessors_and_dispatch() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = filled_service();

    let api = svc.connect(&push);

    assert!(!api.is_open());
    assert_eq!(
        api.selected_value(),
        Some(&datetime(2024, 3, 15, 14, 30, 0))
    );

    api.open();
    api.close();
    api.toggle();
    api.on_focusin();
    api.on_focusout(true);
    api.on_segment_focus(DateSegmentKind::Hour);

    assert_eq!(
        sent.borrow().as_slice(),
        &[
            Event::Open,
            Event::Close,
            Event::Toggle,
            Event::FocusIn,
            Event::FocusOut,
            Event::FocusSegment(DateSegmentKind::Hour),
        ]
    );
}

#[test]
fn on_focusout_without_leaving_does_not_send() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_focusout(false);

    assert!(sent.borrow().is_empty());
}

#[test]
fn on_trigger_keydown_enter_and_arrow_down() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_trigger_keydown(&keyboard(KeyboardKey::Enter));
    api.on_trigger_keydown(&keyboard(KeyboardKey::ArrowDown));

    assert_eq!(sent.borrow().as_slice(), &[Event::Toggle, Event::Open]);
}

#[test]
fn on_segment_keydown_alt_arrow_down_opens() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    let mut data = keyboard(KeyboardKey::ArrowDown);

    data.alt_key = true;

    api.on_segment_keydown(DateSegmentKind::Hour, &data, Direction::Ltr);

    assert_eq!(sent.borrow().as_slice(), &[Event::Open]);
}

#[test]
fn on_segment_keydown_backspace_clears_segment() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_segment_keydown(
        DateSegmentKind::Minute,
        &keyboard(KeyboardKey::Backspace),
        Direction::Ltr,
    );

    assert_eq!(
        sent.borrow().as_slice(),
        &[Event::ClearSegment {
            segment: DateSegmentKind::Minute,
        }]
    );
}

// ────────────────────────────────────────────────────────────────────
// Codex review #708: calendar fidelity, form button types, range guard
// ────────────────────────────────────────────────────────────────────

#[test]
fn assemble_uses_configured_calendar_system() {
    let mut svc = service_with(props().calendar(CalendarSystem::Buddhist), en_us());

    // Buddhist year 2567 == Gregorian 2024.
    drop(svc.send(Event::SegmentChange {
        segment: DateSegmentKind::Year,
        value: 2567,
    }));
    drop(svc.send(Event::SegmentChange {
        segment: DateSegmentKind::Month,
        value: 3,
    }));
    drop(svc.send(Event::SegmentChange {
        segment: DateSegmentKind::Day,
        value: 15,
    }));

    let assembled = svc.context().date_value.as_ref().expect("date assembled");
    assert_eq!(assembled.calendar(), CalendarSystem::Buddhist);
}

#[test]
fn hidden_input_serializes_iso_for_non_gregorian_value() {
    // A Buddhist date equivalent to ISO 2024-03-15.
    let buddhist_date = CalendarDate::new(
        CalendarSystem::Buddhist,
        &CalendarDateFields {
            year: Some(2567),
            month: Some(3),
            day: Some(15),
            ..CalendarDateFields::default()
        },
    )
    .expect("valid Buddhist date");
    let value = CalendarDateTime::new(buddhist_date, time(14, 30, 0));

    let svc = service_with(
        Props {
            default_value: Some(value),
            ..props()
        },
        en_us(),
    );
    let api = svc.connect(&|_| {});

    // The hidden input carries the canonical ISO datetime, not the Buddhist year.
    assert_eq!(
        attr(&api.hidden_input_attrs(), HtmlAttr::Value).as_deref(),
        Some("2024-03-15T14:30:00")
    );
}

#[test]
fn trigger_disabled_when_readonly() {
    let svc = service_with(props().readonly(true), en_us());
    let api = svc.connect(&|_| {});
    let attrs = api.trigger_attrs();
    assert_eq!(
        attr(&attrs, HtmlAttr::Aria(AriaAttr::Disabled)).as_deref(),
        Some("true")
    );
}

#[test]
fn trigger_and_clear_have_button_type() {
    let svc = filled_service();
    let api = svc.connect(&|_| {});
    assert_eq!(
        attr(&api.trigger_attrs(), HtmlAttr::Type).as_deref(),
        Some("button")
    );
    assert_eq!(
        attr(&api.clear_trigger_attrs(), HtmlAttr::Type).as_deref(),
        Some("button")
    );
}

#[test]
fn calendar_select_rejects_out_of_range_date() {
    let mut svc = service_with(
        Props {
            min_value: Some(datetime(2024, 1, 1, 0, 0, 0)),
            max_value: Some(datetime(2024, 12, 31, 23, 59, 59)),
            ..props()
        },
        en_us(),
    );
    drop(svc.send(Event::Open));

    // A selection before the minimum date is rejected outright.
    let result = svc.send(Event::CalendarSelectDate(date(2023, 6, 1)));

    assert!(!result.context_changed);
    assert_eq!(svc.context().date_value, None);
    assert_eq!(*svc.state(), State::Open);

    // A selection after the maximum date is likewise rejected.
    drop(svc.send(Event::CalendarSelectDate(date(2025, 6, 1))));
    assert_eq!(svc.context().date_value, None);

    // An in-range selection is accepted.
    drop(svc.send(Event::CalendarSelectDate(date(2024, 6, 1))));
    assert_eq!(svc.context().date_value, Some(date(2024, 6, 1)));
}

// ────────────────────────────────────────────────────────────────────
// Codex review #708 (pass 2): calendar fidelity, init clamp, dialog name,
// dynamic day range
// ────────────────────────────────────────────────────────────────────

#[test]
fn init_clamps_out_of_range_default_value() {
    let svc = service_with(
        Props {
            default_value: Some(datetime(2020, 1, 1, 0, 0, 0)),
            min_value: Some(datetime(2024, 6, 1, 9, 0, 0)),
            max_value: Some(datetime(2024, 12, 31, 17, 0, 0)),
            ..props()
        },
        en_us(),
    );

    // The out-of-range default is clamped at mount, so the hidden input never
    // exposes a disallowed datetime.
    assert_eq!(
        *svc.context().value.get(),
        Some(datetime(2024, 6, 1, 9, 0, 0))
    );
    let api = svc.connect(&|_| {});
    assert_eq!(
        attr(&api.hidden_input_attrs(), HtmlAttr::Value).as_deref(),
        Some("2024-06-01T09:00:00")
    );
}

#[test]
fn content_dialog_has_accessible_name() {
    let svc = service();
    let api = svc.connect(&|_| {});
    assert_eq!(
        attr(&api.content_attrs(), HtmlAttr::Aria(AriaAttr::Label)).as_deref(),
        Some("Choose date and time")
    );
}

#[test]
fn calendar_props_project_dates_into_configured_calendar() {
    let svc = service_with(
        Props {
            calendar: CalendarSystem::Buddhist,
            min_value: Some(datetime(2024, 1, 1, 0, 0, 0)),
            max_value: Some(datetime(2024, 12, 31, 0, 0, 0)),
            today: date(2024, 3, 1),
            ..props()
        },
        en_us(),
    );
    let api = svc.connect(&|_| {});
    let calendar_props = api.calendar_props();

    // `today` and the min/max bounds reach the embedded calendar in the
    // configured (Buddhist) calendar, not Gregorian.
    assert_eq!(calendar_props.today.calendar(), CalendarSystem::Buddhist);
    assert_eq!(
        calendar_props.min.map(|date| date.calendar()),
        Some(CalendarSystem::Buddhist)
    );
    assert_eq!(
        calendar_props.max.map(|date| date.calendar()),
        Some(CalendarSystem::Buddhist)
    );
}

#[test]
fn sync_props_calendar_change_reprojects_date_value() {
    let mut svc = service_with(
        Props {
            default_value: Some(datetime(2024, 3, 15, 14, 0, 0)),
            ..props()
        },
        en_us(),
    );
    assert_eq!(
        svc.context()
            .date_value
            .as_ref()
            .map(CalendarDate::calendar),
        Some(CalendarSystem::Gregorian)
    );

    drop(svc.send(Event::SyncProps(Box::new(Props {
        calendar: CalendarSystem::Buddhist,
        default_value: Some(datetime(2024, 3, 15, 14, 0, 0)),
        ..props()
    }))));

    // The displayed date is re-projected into Buddhist (year 2567), so future
    // edits assemble in the right calendar instead of reusing stale fields.
    let date_value = svc.context().date_value.as_ref().expect("date present");
    assert_eq!(date_value.calendar(), CalendarSystem::Buddhist);
    assert_eq!(date_value.year(), 2567);
}

#[test]
fn day_range_follows_month_length() {
    // February 2024 is a leap month (29 days): the day segment max is 29 and
    // incrementing the last day wraps to 1 rather than showing an impossible 30.
    let mut svc = service_with(
        Props {
            default_value: Some(datetime(2024, 2, 29, 12, 0, 0)),
            ..props()
        },
        en_us(),
    );
    let day_max = svc
        .context()
        .segment(DateSegmentKind::Day)
        .map(|segment| segment.max);
    assert_eq!(day_max, Some(29));

    drop(svc.send(Event::IncrementSegment {
        segment: DateSegmentKind::Day,
    }));
    assert_eq!(svc.context().segment_value(DateSegmentKind::Day), Some(1));
    // The committed value stays consistent with the visible day.
    assert_eq!(
        svc.context().value.get().as_ref().map(|dt| dt.date().day()),
        Some(1)
    );
}

#[test]
fn day_range_clamps_when_month_shortens() {
    let mut svc = service();

    // Build up 2023-01-31, then switch the month to February (28 days, non-leap):
    // the day clamps from 31 down to 28.
    drop(svc.send(Event::SegmentChange {
        segment: DateSegmentKind::Year,
        value: 2023,
    }));
    drop(svc.send(Event::SegmentChange {
        segment: DateSegmentKind::Day,
        value: 31,
    }));
    drop(svc.send(Event::SegmentChange {
        segment: DateSegmentKind::Month,
        value: 2,
    }));

    assert_eq!(svc.context().segment_value(DateSegmentKind::Day), Some(28));
}

// ────────────────────────────────────────────────────────────────────
// Codex review #708 (pass 3): reprojection, era, leap months, disabled input
// ────────────────────────────────────────────────────────────────────

#[test]
fn maybe_publish_reprojects_clamped_date_into_calendar() {
    // Buddhist picker whose value is clamped to a Gregorian min bound on edit.
    let buddhist = date(2024, 6, 15)
        .to_calendar(CalendarSystem::Buddhist)
        .expect("buddhist date");
    let mut svc = service_with(
        Props {
            calendar: CalendarSystem::Buddhist,
            default_value: Some(CalendarDateTime::new(buddhist, time(12, 0, 0))),
            min_value: Some(datetime(2024, 6, 15, 13, 0, 0)),
            ..props()
        },
        en_us(),
    );

    // Re-publish below the min so `clamp_datetime` returns the Gregorian bound;
    // the cached display date must still be Buddhist.
    drop(svc.send(Event::SegmentChange {
        segment: DateSegmentKind::Minute,
        value: 0,
    }));

    assert_eq!(
        svc.context()
            .date_value
            .as_ref()
            .map(CalendarDate::calendar),
        Some(CalendarSystem::Buddhist)
    );
}

#[test]
fn value_commit_reprojects_into_configured_calendar() {
    let mut svc = service_with(props().calendar(CalendarSystem::Buddhist), en_us());

    // A canonical (Gregorian-shaped) commit is reprojected into Buddhist.
    drop(svc.send(Event::ValueCommit(Some(datetime(2024, 3, 15, 14, 30, 0)))));

    let date_value = svc.context().date_value.as_ref().expect("date present");
    assert_eq!(date_value.calendar(), CalendarSystem::Buddhist);
    assert_eq!(date_value.year(), 2567);
}

#[test]
fn editing_preserves_japanese_era() {
    // Reiwa 6 == Gregorian 2024.
    let japanese = date(2024, 3, 15)
        .to_calendar(CalendarSystem::Japanese)
        .expect("japanese date");
    let mut svc = service_with(
        Props {
            calendar: CalendarSystem::Japanese,
            default_value: Some(CalendarDateTime::new(japanese, time(14, 30, 0))),
            ..props()
        },
        en_us(),
    );

    // Editing the day keeps the Reiwa era, so the ISO year stays 2024 rather
    // than reinterpreting the era-year `6` as ISO year 0006.
    drop(svc.send(Event::SegmentChange {
        segment: DateSegmentKind::Day,
        value: 16,
    }));

    let api = svc.connect(&|_| {});
    assert_eq!(
        attr(&api.hidden_input_attrs(), HtmlAttr::Value).as_deref(),
        Some("2024-03-16T14:30:00")
    );
}

#[test]
fn month_range_follows_calendar_leap_year() {
    // Hebrew year 5784 (≈ 2024) is a leap year with 13 months.
    let hebrew = date(2024, 3, 15)
        .to_calendar(CalendarSystem::Hebrew)
        .expect("hebrew date");
    let svc = service_with(
        Props {
            calendar: CalendarSystem::Hebrew,
            default_value: Some(CalendarDateTime::new(hebrew, time(12, 0, 0))),
            ..props()
        },
        en_us(),
    );

    let month_max = svc
        .context()
        .segment(DateSegmentKind::Month)
        .map(|segment| segment.max);
    assert_eq!(month_max, Some(13));
}

#[test]
fn month_segment_announces_localized_name() {
    let svc = service_with(
        Props {
            default_value: Some(datetime(2024, 3, 15, 14, 30, 0)),
            ..props()
        },
        en_us(),
    );
    let api = svc.connect(&|_| {});
    // aria-valuetext announces the month name, not the padded "03".
    assert_eq!(
        attr(
            &api.segment_attrs(&DateSegmentKind::Month),
            HtmlAttr::Aria(AriaAttr::ValueText)
        )
        .as_deref(),
        Some("March")
    );
}

#[test]
fn day_period_typing_routes_through_backend() {
    // The backend maps `a`/`p` (and localized labels) to AM/PM.
    let mut svc = service();
    drop(svc.send(Event::TypeIntoSegment {
        segment: DateSegmentKind::DayPeriod,
        ch: 'p',
    }));
    assert_eq!(
        svc.context().segment_value(DateSegmentKind::DayPeriod),
        Some(1)
    );
}

#[test]
fn sync_controlled_to_uncontrolled_clears_stale_value() {
    let mut svc = service_with(
        Props {
            value: Some(Some(datetime(2024, 3, 15, 14, 30, 0))),
            ..props()
        },
        en_us(),
    );
    assert_eq!(
        *svc.context().value.get(),
        Some(datetime(2024, 3, 15, 14, 30, 0))
    );

    // Parent relinquishes control (Some(..) -> None): the staged value clears.
    drop(svc.send(Event::SyncProps(Box::new(props()))));

    assert_eq!(*svc.context().value.get(), None);
    assert_eq!(svc.context().date_value, None);
    let api = svc.connect(&|_| {});
    assert_eq!(attr(&api.hidden_input_attrs(), HtmlAttr::Value), None);
}

#[test]
fn focus_next_past_last_segment_clears_focus_and_targets_trigger() {
    let mut svc = service();

    // DayPeriod is the last editable segment in en-US (H12).
    drop(svc.send(Event::FocusSegment(DateSegmentKind::DayPeriod)));
    let result = svc.send(Event::FocusNextSegment);

    assert!(effects(result).contains(&Effect::RestoreFocusToTrigger));
    assert_eq!(svc.context().focused_segment, None);
}

#[test]
fn sync_props_disable_while_open_clears_open_flag() {
    let mut svc = service();
    drop(svc.send(Event::Open));
    assert!(svc.context().open);

    drop(svc.send(Event::SyncProps(Box::new(props().disabled(true)))));

    assert_eq!(*svc.state(), State::Idle);
    // The public `open` flag must track the reconciled state.
    assert!(!svc.context().open);
}

#[test]
fn hidden_input_disabled_when_picker_disabled() {
    let svc = service_with(
        Props {
            disabled: true,
            name: Some(String::from("appointment")),
            default_value: Some(datetime(2024, 3, 15, 14, 30, 0)),
            ..props()
        },
        en_us(),
    );
    let api = svc.connect(&|_| {});
    assert_eq!(
        attr(&api.hidden_input_attrs(), HtmlAttr::Disabled).as_deref(),
        Some("true")
    );
}

// ────────────────────────────────────────────────────────────────────
// Additional branch coverage: keydown guards, type-ahead edges, sync, format
// ────────────────────────────────────────────────────────────────────

#[test]
fn on_segment_keydown_plain_arrow_down_decrements() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_segment_keydown(
        DateSegmentKind::Hour,
        &keyboard(KeyboardKey::ArrowDown),
        Direction::Ltr,
    );

    assert_eq!(
        sent.borrow().as_slice(),
        &[Event::DecrementSegment {
            segment: DateSegmentKind::Hour,
        }]
    );
}

#[test]
fn on_segment_keydown_unhandled_key_sends_nothing() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    // `Home` is not handled and carries no character → no event dispatched.
    api.on_segment_keydown(
        DateSegmentKind::Hour,
        &keyboard(KeyboardKey::Home),
        Direction::Ltr,
    );

    assert!(sent.borrow().is_empty());
}

#[test]
fn on_content_keydown_non_escape_sends_nothing() {
    let sent = RefCell::new(Vec::new());
    let push = |event| sent.borrow_mut().push(event);

    let svc = service();

    let api = svc.connect(&push);

    api.on_content_keydown(&keyboard(KeyboardKey::Enter));

    assert!(sent.borrow().is_empty());
}

#[test]
fn type_into_numeric_segment_ignores_non_digit() {
    let mut svc = service();

    drop(svc.send(Event::FocusSegment(DateSegmentKind::Month)));

    let result = svc.send(Event::TypeIntoSegment {
        segment: DateSegmentKind::Month,
        ch: 'x',
    });

    assert!(!result.context_changed);
    assert_eq!(svc.context().segment_value(DateSegmentKind::Month), None);
}

#[test]
fn type_into_hour_rejects_out_of_range_zero() {
    let mut svc = service();

    drop(svc.send(Event::FocusSegment(DateSegmentKind::Hour)));

    // Hour (H12) range is 1..=12; a buffered `0` is below the minimum and is
    // not committed, though it stays buffered for a following digit.
    drop(svc.send(Event::TypeIntoSegment {
        segment: DateSegmentKind::Hour,
        ch: '0',
    }));

    assert_eq!(svc.context().segment_value(DateSegmentKind::Hour), None);
}

#[test]
fn value_commit_on_controlled_value_syncs_through_bindable() {
    let mut svc = service_with(
        Props {
            value: Some(Some(datetime(2024, 1, 1, 0, 0, 0))),
            ..props()
        },
        en_us(),
    );

    drop(svc.send(Event::ValueCommit(Some(datetime(2024, 3, 15, 14, 30, 0)))));

    assert_eq!(
        *svc.context().value.get(),
        Some(datetime(2024, 3, 15, 14, 30, 0))
    );
    assert_eq!(svc.context().date_value, Some(date(2024, 3, 15)));
}

#[test]
fn sync_props_granularity_change_rebuilds_time_segments() {
    let mut svc = service();

    assert!(
        !svc.context()
            .time_segments
            .iter()
            .any(|segment| segment.kind == DateSegmentKind::Second)
    );

    drop(svc.send(Event::SyncProps(Box::new(
        props().granularity(TimeGranularity::Second),
    ))));

    assert!(
        svc.context()
            .time_segments
            .iter()
            .any(|segment| segment.kind == DateSegmentKind::Second)
    );
}

#[test]
fn hour_granularity_omits_minute_and_second_segments() {
    let svc = service_with(props().granularity(TimeGranularity::Hour), en_us());

    let kinds = segment_kinds(svc.context().time_segments.as_slice());

    assert!(kinds.contains(&DateSegmentKind::Hour));
    assert!(!kinds.contains(&DateSegmentKind::Minute));
    assert!(!kinds.contains(&DateSegmentKind::Second));
}

#[test]
fn readonly_blocks_every_editing_event() {
    let editing = [
        Event::CalendarSelectDate(date(2024, 3, 15)),
        Event::IncrementSegment {
            segment: DateSegmentKind::Month,
        },
        Event::DecrementSegment {
            segment: DateSegmentKind::Month,
        },
        Event::TypeIntoSegment {
            segment: DateSegmentKind::Month,
            ch: '3',
        },
        Event::SegmentChange {
            segment: DateSegmentKind::Month,
            value: 3,
        },
        Event::ValueCommit(Some(datetime(2024, 3, 15, 14, 30, 0))),
        Event::ClearSegment {
            segment: DateSegmentKind::Month,
        },
        Event::ClearAll,
    ];

    for event in editing {
        let mut svc = service_with(props().readonly(true), en_us());

        let result = svc.send(event);

        assert!(!result.context_changed, "read-only must block editing");
        assert_eq!(*svc.context().value.get(), None);
    }
}

#[test]
fn focus_in_when_already_focused_is_noop() {
    let mut svc = service();

    drop(svc.send(Event::FocusIn));

    assert_eq!(*svc.state(), State::Focused);

    let result = svc.send(Event::FocusIn);

    assert!(!result.state_changed);
}

#[test]
fn focus_out_closes_open_popover() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    assert!(svc.context().open);

    drop(svc.send(Event::FocusOut));

    assert_eq!(*svc.state(), State::Idle);
    assert!(!svc.context().open);
}

#[test]
fn escape_when_closed_is_noop() {
    let mut svc = service();

    let result = svc.send(Event::KeyDown {
        key: KeyboardKey::Escape,
    });

    assert!(!result.state_changed);
}

#[test]
fn sync_props_tightened_min_clamps_existing_value() {
    let mut svc = service_with(
        Props {
            default_value: Some(datetime(2024, 3, 15, 8, 0, 0)),
            ..props()
        },
        en_us(),
    );

    // Raising the minimum above the current value re-clamps it (value_changed).
    drop(svc.send(Event::SyncProps(Box::new(Props {
        default_value: Some(datetime(2024, 3, 15, 8, 0, 0)),
        min_value: Some(datetime(2024, 3, 16, 0, 0, 0)),
        ..props()
    }))));

    assert_eq!(
        *svc.context().value.get(),
        Some(datetime(2024, 3, 16, 0, 0, 0))
    );
}

#[test]
fn day_period_increment_decrement_from_empty() {
    let mut increment = service();

    drop(increment.send(Event::IncrementSegment {
        segment: DateSegmentKind::DayPeriod,
    }));

    assert_eq!(
        increment
            .context()
            .segment_value(DateSegmentKind::DayPeriod),
        Some(1)
    );

    let mut decrement = service();

    drop(decrement.send(Event::DecrementSegment {
        segment: DateSegmentKind::DayPeriod,
    }));

    assert_eq!(
        decrement
            .context()
            .segment_value(DateSegmentKind::DayPeriod),
        Some(0)
    );
}

#[test]
fn focus_segment_on_non_editable_kind_is_noop() {
    let mut svc = service();

    // `Era` is never built into the date-time picker's segment lists.
    let result = svc.send(Event::FocusSegment(DateSegmentKind::Era));

    assert!(!result.state_changed);
    assert_eq!(svc.context().focused_segment, None);
}

#[test]
fn clear_trigger_disabled_when_readonly() {
    let svc = service_with(props().readonly(true), en_us());

    let api = svc.connect(&|_| {});

    let attrs = api.clear_trigger_attrs();

    assert_eq!(
        attr(&attrs, HtmlAttr::Aria(AriaAttr::Disabled)).as_deref(),
        Some("true")
    );
}

#[test]
fn control_describedby_omits_error_when_valid() {
    // `invalid = false` (with an error message present) must not chain the
    // error-message id — only the description.
    let svc = service_with(
        Props {
            description: Some(String::from("help")),
            error_message: Some(String::from("bad")),
            invalid: false,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.control_attrs(), HtmlAttr::Aria(AriaAttr::DescribedBy)).as_deref(),
        Some("date-time-picker-description")
    );
}

#[test]
fn sync_props_disable_while_open_returns_to_idle() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    assert_eq!(*svc.state(), State::Open);

    drop(svc.send(Event::SyncProps(Box::new(props().disabled(true)))));

    assert_eq!(*svc.state(), State::Idle);
}

#[test]
fn hour_granularity_publish_keeps_minute_zero() {
    let mut svc = service_with(
        Props {
            granularity: TimeGranularity::Hour,
            default_value: Some(datetime(2024, 3, 15, 14, 0, 0)),
            ..props()
        },
        en_us(),
    );

    // Re-publishing under Hour granularity assembles minute/second as zero.
    drop(svc.send(Event::IncrementSegment {
        segment: DateSegmentKind::Hour,
    }));

    assert_eq!(
        *svc.context().value.get(),
        Some(datetime(2024, 3, 15, 15, 0, 0))
    );
}

#[test]
fn announcement_uses_am_for_morning_times() {
    let svc = service_with(
        Props {
            default_value: Some(datetime(2024, 3, 15, 9, 5, 0)),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_eq!(
        attr(&api.root_attrs(), HtmlAttr::Aria(AriaAttr::Description)).as_deref(),
        Some("Selected: 03/15/2024 09:05 AM")
    );
}

// ────────────────────────────────────────────────────────────────────
// Snapshots
// ────────────────────────────────────────────────────────────────────

fn filled_service() -> Service<Machine> {
    service_with(
        Props {
            default_value: Some(datetime(2024, 3, 15, 14, 30, 0)),
            ..props()
        },
        en_us(),
    )
}

#[test]
fn snapshot_root_idle() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_idle", snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_open() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_open", snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_with_value() {
    let svc = filled_service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_with_value", snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_disabled() {
    let svc = service_with(props().disabled(true), en_us());

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_disabled", snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_readonly() {
    let svc = service_with(props().readonly(true), en_us());

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_readonly", snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_invalid() {
    let svc = service_with(props().invalid(true), en_us());

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_invalid", snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_root_rtl() {
    let svc = service_with(props().is_rtl(true), en_us());

    let api = svc.connect(&|_| {});

    assert_snapshot!("root_rtl", snapshot_attrs(&api.root_attrs()));
}

#[test]
fn snapshot_label() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("label", snapshot_attrs(&api.label_attrs()));
}

#[test]
fn snapshot_control() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("control", snapshot_attrs(&api.control_attrs()));
}

#[test]
fn snapshot_control_described() {
    let svc = service_with(
        Props {
            description: Some(String::from("Pick a slot")),
            error_message: Some(String::from("Required")),
            invalid: true,
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!("control_described", snapshot_attrs(&api.control_attrs()));
}

#[test]
fn snapshot_date_segment_group() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "date_segment_group",
        snapshot_attrs(&api.date_segment_group_attrs())
    );
}

#[test]
fn snapshot_date_segment_group_required_invalid() {
    let svc = service_with(props().required(true).invalid(true), en_us());

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "date_segment_group_required_invalid",
        snapshot_attrs(&api.date_segment_group_attrs())
    );
}

#[test]
fn snapshot_time_segment_group() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "time_segment_group",
        snapshot_attrs(&api.time_segment_group_attrs())
    );
}

#[test]
fn snapshot_segment_month_empty() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "segment_month_empty",
        snapshot_attrs(&api.segment_attrs(&DateSegmentKind::Month))
    );
}

#[test]
fn snapshot_segment_year_set() {
    let svc = filled_service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "segment_year_set",
        snapshot_attrs(&api.segment_attrs(&DateSegmentKind::Year))
    );
}

#[test]
fn snapshot_segment_hour_h12_set() {
    let svc = filled_service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "segment_hour_h12_set",
        snapshot_attrs(&api.segment_attrs(&DateSegmentKind::Hour))
    );
}

#[test]
fn snapshot_segment_hour_h23() {
    let svc = service_with(props(), de_de());

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "segment_hour_h23",
        snapshot_attrs(&api.segment_attrs(&DateSegmentKind::Hour))
    );
}

#[test]
fn snapshot_segment_day_period_pm() {
    let svc = filled_service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "segment_day_period_pm",
        snapshot_attrs(&api.segment_attrs(&DateSegmentKind::DayPeriod))
    );
}

#[test]
fn snapshot_segment_focused() {
    let mut svc = service();

    drop(svc.send(Event::FocusSegment(DateSegmentKind::Month)));

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "segment_focused",
        snapshot_attrs(&api.segment_attrs(&DateSegmentKind::Month))
    );
}

#[test]
fn snapshot_segment_disabled() {
    let svc = service_with(props().disabled(true), en_us());

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "segment_disabled",
        snapshot_attrs(&api.segment_attrs(&DateSegmentKind::Month))
    );
}

#[test]
fn snapshot_literal() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("literal", snapshot_attrs(&api.literal_attrs(1)));
}

#[test]
fn snapshot_separator() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("separator", snapshot_attrs(&api.separator_attrs()));
}

#[test]
fn snapshot_trigger_closed() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("trigger_closed", snapshot_attrs(&api.trigger_attrs()));
}

#[test]
fn snapshot_trigger_open() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let api = svc.connect(&|_| {});

    assert_snapshot!("trigger_open", snapshot_attrs(&api.trigger_attrs()));
}

#[test]
fn snapshot_trigger_disabled() {
    let svc = service_with(props().disabled(true), en_us());

    let api = svc.connect(&|_| {});

    assert_snapshot!("trigger_disabled", snapshot_attrs(&api.trigger_attrs()));
}

#[test]
fn snapshot_clear_trigger_empty() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "clear_trigger_empty",
        snapshot_attrs(&api.clear_trigger_attrs())
    );
}

#[test]
fn snapshot_clear_trigger_with_value() {
    let svc = filled_service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "clear_trigger_with_value",
        snapshot_attrs(&api.clear_trigger_attrs())
    );
}

#[test]
fn snapshot_positioner_closed() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("positioner_closed", snapshot_attrs(&api.positioner_attrs()));
}

#[test]
fn snapshot_positioner_open() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let api = svc.connect(&|_| {});

    assert_snapshot!("positioner_open", snapshot_attrs(&api.positioner_attrs()));
}

#[test]
fn snapshot_content() {
    let mut svc = service();

    drop(svc.send(Event::Open));

    let api = svc.connect(&|_| {});

    assert_snapshot!("content", snapshot_attrs(&api.content_attrs()));
}

#[test]
fn snapshot_description() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("description", snapshot_attrs(&api.description_attrs()));
}

#[test]
fn snapshot_error_message() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!("error_message", snapshot_attrs(&api.error_message_attrs()));
}

#[test]
fn snapshot_hidden_input_empty() {
    let svc = service();

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "hidden_input_empty",
        snapshot_attrs(&api.hidden_input_attrs())
    );
}

#[test]
fn snapshot_hidden_input_with_value() {
    let svc = service_with(
        Props {
            name: Some(String::from("appointment")),
            default_value: Some(datetime(2024, 3, 15, 14, 30, 0)),
            ..props()
        },
        en_us(),
    );

    let api = svc.connect(&|_| {});

    assert_snapshot!(
        "hidden_input_with_value",
        snapshot_attrs(&api.hidden_input_attrs())
    );
}
