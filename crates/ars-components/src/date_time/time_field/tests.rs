use alloc::{string::String, sync::Arc};
use core::cell::RefCell;

use ars_core::{
    AriaAttr, AttrMap, ConnectApi, Direction, Env, HtmlAttr, KeyboardKey, Service, StubIntlBackend,
};
use ars_i18n::{HourCycle, Locale, Time};
use ars_interactions::KeyboardEventData;
use insta::assert_snapshot;

use super::*;
use crate::date_time::date_field::DateSegmentKind;

fn time(hour: u8, minute: u8, second: u8) -> Time {
    Time::new(hour, minute, second, 0).expect("test time should be valid")
}

fn props() -> Props {
    Props::new().id("meeting-time").label("Meeting time")
}

fn service() -> Service<Machine> {
    Service::<Machine>::new(props(), &Env::default(), &Messages::default())
}

fn env(locale: &str) -> Env {
    Env::new(
        Locale::parse(locale).expect("test locale should parse"),
        Arc::new(StubIntlBackend),
    )
}

fn snapshot_attrs(attrs: &AttrMap) -> String {
    format!("{attrs:#?}")
}

fn key_data(key: KeyboardKey, character: Option<char>) -> KeyboardEventData {
    KeyboardEventData {
        key,
        character,
        code: String::new(),
        shift_key: false,
        ctrl_key: false,
        alt_key: false,
        meta_key: false,
        repeat: false,
        is_composing: false,
    }
}

fn attr(attrs: &AttrMap, name: HtmlAttr) -> Option<&str> {
    attrs.get(&name)
}

#[test]
fn props_builder_sets_every_field() {
    let value = time(14, 30, 45);
    let default_value = time(9, 5, 0);
    let min_value = time(8, 0, 0);
    let max_value = time(18, 0, 0);

    let props = Props::new()
        .id("time")
        .value(Some(value))
        .default_value(Some(default_value))
        .granularity(TimeGranularity::Second)
        .hour_cycle(Some(HourCycle::H12))
        .hide_time_zone(true)
        .disabled(true)
        .readonly(true)
        .required(true)
        .min_value(Some(min_value))
        .max_value(Some(max_value))
        .label("Time")
        .aria_label(Some(String::from("Appointment time")))
        .aria_describedby(Some(String::from("help-id")))
        .description(Some(String::from("Help")))
        .error_message(Some(String::from("Error")))
        .invalid(true)
        .name(Some(String::from("appointment_time")))
        .force_leading_zeros(true);

    assert_eq!(props.id, "time");
    assert_eq!(props.value, Some(value));
    assert_eq!(props.default_value, Some(default_value));
    assert_eq!(props.granularity, TimeGranularity::Second);
    assert_eq!(props.hour_cycle, Some(HourCycle::H12));
    assert!(props.hide_time_zone);
    assert!(props.disabled);
    assert!(props.readonly);
    assert!(props.required);
    assert_eq!(props.min_value, Some(min_value));
    assert_eq!(props.max_value, Some(max_value));
    assert_eq!(props.label, "Time");
    assert_eq!(props.aria_label.as_deref(), Some("Appointment time"));
    assert_eq!(props.aria_describedby.as_deref(), Some("help-id"));
    assert_eq!(props.description.as_deref(), Some("Help"));
    assert_eq!(props.error_message.as_deref(), Some("Error"));
    assert!(props.invalid);
    assert_eq!(props.name.as_deref(), Some("appointment_time"));
    assert!(props.force_leading_zeros);
}

#[test]
fn per_segment_navigation_moves_through_visible_segments() {
    let mut service = Service::<Machine>::new(
        props()
            .granularity(TimeGranularity::Second)
            .hour_cycle(Some(HourCycle::H12)),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FocusSegment {
        kind: DateSegmentKind::Hour,
    }));

    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Hour));

    drop(service.send(Event::FocusNextSegment));

    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Minute));

    drop(service.send(Event::FocusNextSegment));

    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Second));

    drop(service.send(Event::FocusNextSegment));

    assert_eq!(service.state(), &State::Focused(DateSegmentKind::DayPeriod));

    drop(service.send(Event::FocusPrevSegment));

    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Second));
}

#[test]
fn keydown_respects_ltr_and_rtl_arrow_direction() {
    let service = service();

    let sent = RefCell::new(Vec::new());
    let send = |event| sent.borrow_mut().push(event);

    let api = service.connect(&send);

    api.on_segment_keydown(
        DateSegmentKind::Hour,
        &key_data(KeyboardKey::ArrowRight, None),
        false,
        Direction::Ltr,
    );
    api.on_segment_keydown(
        DateSegmentKind::Hour,
        &key_data(KeyboardKey::ArrowRight, None),
        false,
        Direction::Rtl,
    );

    assert_eq!(sent.borrow()[0], Event::FocusNextSegment);
    assert_eq!(sent.borrow()[1], Event::FocusPrevSegment);
}

#[test]
fn locale_hour_cycle_controls_visible_segments() {
    let us = Service::<Machine>::new(props(), &env("en-US"), &Messages::default());

    assert_eq!(us.context().hour_cycle, HourCycle::H12);
    assert!(
        us.context()
            .segments
            .iter()
            .any(|segment| segment.kind == DateSegmentKind::DayPeriod)
    );

    let de = Service::<Machine>::new(props(), &env("de-DE"), &Messages::default());

    assert_eq!(de.context().hour_cycle, HourCycle::H23);
    assert!(
        !de.context()
            .segments
            .iter()
            .any(|segment| segment.kind == DateSegmentKind::DayPeriod)
    );

    let forced = Service::<Machine>::new(
        props().hour_cycle(Some(HourCycle::H23)),
        &env("en-US"),
        &Messages::default(),
    );

    assert_eq!(forced.context().hour_cycle, HourCycle::H23);
}

#[test]
fn am_pm_segment_toggles_and_maps_to_twenty_four_hour_time() {
    let mut service = Service::<Machine>::new(
        props().hour_cycle(Some(HourCycle::H12)),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::IncrementSegment {
        kind: DateSegmentKind::DayPeriod,
    }));

    assert_eq!(
        service
            .context()
            .get_segment_value(DateSegmentKind::DayPeriod),
        Some(1)
    );

    drop(service.send(Event::DecrementSegment {
        kind: DateSegmentKind::DayPeriod,
    }));

    assert_eq!(
        service
            .context()
            .get_segment_value(DateSegmentKind::DayPeriod),
        Some(0)
    );

    drop(service.send(Event::FocusSegment {
        kind: DateSegmentKind::DayPeriod,
    }));
    drop(service.send(Event::TypeIntoSegment {
        kind: DateSegmentKind::DayPeriod,
        ch: 'p',
    }));

    assert_eq!(
        service
            .context()
            .get_segment_value(DateSegmentKind::DayPeriod),
        Some(1)
    );

    let mut ctx = service.context().clone();

    ctx.set_segment_value(DateSegmentKind::Hour, 12);
    ctx.set_segment_value(DateSegmentKind::Minute, 0);
    ctx.set_segment_value(DateSegmentKind::DayPeriod, 0);

    assert_eq!(ctx.assemble_time(), Some(time(0, 0, 0)));

    ctx.set_segment_value(DateSegmentKind::DayPeriod, 1);

    assert_eq!(ctx.assemble_time(), Some(time(12, 0, 0)));
}

#[test]
fn cjk_day_period_input_buffers_ambiguous_prefix_and_commits_disambiguation() {
    let mut service = Service::<Machine>::new(
        props().hour_cycle(Some(HourCycle::H12)),
        &env("ja-JP"),
        &Messages::default(),
    );

    drop(service.send(Event::FocusSegment {
        kind: DateSegmentKind::DayPeriod,
    }));
    drop(service.send(Event::TypeIntoSegment {
        kind: DateSegmentKind::DayPeriod,
        ch: '午',
    }));

    assert_eq!(service.context().type_buffer, "午");
    assert_eq!(
        service
            .context()
            .get_segment_value(DateSegmentKind::DayPeriod),
        None
    );

    drop(service.send(Event::TypeIntoSegment {
        kind: DateSegmentKind::DayPeriod,
        ch: '後',
    }));

    assert_eq!(service.context().type_buffer, "");
    assert_eq!(
        service
            .context()
            .get_segment_value(DateSegmentKind::DayPeriod),
        Some(1)
    );
}

#[test]
fn cjk_day_period_timeout_uses_current_hour_as_fallback() {
    let mut ctx = Service::<Machine>::new(
        props().hour_cycle(Some(HourCycle::H12)),
        &env("ja-JP"),
        &Messages::default(),
    )
    .context()
    .clone();

    ctx.set_segment_value(DateSegmentKind::Hour, 11);
    ctx.type_buffer = String::from("午");

    commit_buffer_for_kind(&mut ctx, DateSegmentKind::DayPeriod, true);

    assert_eq!(ctx.get_segment_value(DateSegmentKind::DayPeriod), Some(0));

    ctx.set_segment_value(DateSegmentKind::Hour, 12);
    ctx.type_buffer = String::from("午");

    commit_buffer_for_kind(&mut ctx, DateSegmentKind::DayPeriod, true);

    assert_eq!(ctx.get_segment_value(DateSegmentKind::DayPeriod), Some(1));
}

#[test]
fn cjk_day_period_timeout_preserves_h11_current_day_period() {
    let mut ctx = Service::<Machine>::new(
        props().hour_cycle(Some(HourCycle::H11)),
        &env("ja-JP"),
        &Messages::default(),
    )
    .context()
    .clone();

    ctx.set_segment_value(DateSegmentKind::Hour, 11);
    ctx.set_segment_value(DateSegmentKind::DayPeriod, 1);
    ctx.type_buffer = String::from("午");

    commit_buffer_for_kind(&mut ctx, DateSegmentKind::DayPeriod, true);

    assert_eq!(ctx.get_segment_value(DateSegmentKind::DayPeriod), Some(1));
}

#[test]
fn connect_api_segment_attrs_include_spinbutton_range_and_value_text() {
    let service = Service::<Machine>::new(
        props()
            .default_value(Some(time(14, 30, 45)))
            .granularity(TimeGranularity::Second)
            .hour_cycle(Some(HourCycle::H12)),
        &Env::default(),
        &Messages::default(),
    );

    let api = service.connect(&|_| {});

    let hour_attrs = api.segment_attrs(&DateSegmentKind::Hour);

    assert_eq!(attr(&hour_attrs, HtmlAttr::Role), Some("spinbutton"));
    assert_eq!(
        attr(&hour_attrs, HtmlAttr::Aria(AriaAttr::ValueMin)),
        Some("1")
    );
    assert_eq!(
        attr(&hour_attrs, HtmlAttr::Aria(AriaAttr::ValueMax)),
        Some("12")
    );
    assert_eq!(
        attr(&hour_attrs, HtmlAttr::Aria(AriaAttr::ValueNow)),
        Some("2")
    );
    assert_eq!(
        attr(&hour_attrs, HtmlAttr::Aria(AriaAttr::ValueText)),
        Some("2")
    );

    let period_attrs = api.segment_attrs(&DateSegmentKind::DayPeriod);

    assert_eq!(
        attr(&period_attrs, HtmlAttr::Aria(AriaAttr::ValueText)),
        Some("PM")
    );
}

#[test]
fn segment_bounds_are_enforced_by_hour_cycle_and_time_units() {
    let mut h23 = Service::<Machine>::new(
        props().hour_cycle(Some(HourCycle::H23)),
        &Env::default(),
        &Messages::default(),
    )
    .context()
    .clone();

    h23.set_segment_value(DateSegmentKind::Hour, 99);

    assert_eq!(h23.get_segment_value(DateSegmentKind::Hour), Some(23));

    h23.decrement_segment(DateSegmentKind::Hour);

    assert_eq!(h23.get_segment_value(DateSegmentKind::Hour), Some(22));

    let mut h12 = Service::<Machine>::new(
        props().hour_cycle(Some(HourCycle::H12)),
        &Env::default(),
        &Messages::default(),
    )
    .context()
    .clone();

    h12.set_segment_value(DateSegmentKind::Hour, 0);

    assert_eq!(h12.get_segment_value(DateSegmentKind::Hour), Some(1));

    let mut h11 = Service::<Machine>::new(
        props().hour_cycle(Some(HourCycle::H11)),
        &Env::default(),
        &Messages::default(),
    )
    .context()
    .clone();

    h11.set_segment_value(DateSegmentKind::Hour, 99);

    assert_eq!(h11.get_segment_value(DateSegmentKind::Hour), Some(11));

    h12.set_segment_value(DateSegmentKind::Minute, 99);
    h12.set_segment_value(DateSegmentKind::Second, 99);

    assert_eq!(h12.get_segment_value(DateSegmentKind::Minute), Some(59));
    assert_eq!(h12.get_segment_value(DateSegmentKind::Second), None);
}

#[test]
fn granularity_controls_visible_segments() {
    for (granularity, expected) in [
        (
            TimeGranularity::Hour,
            vec![DateSegmentKind::Hour, DateSegmentKind::DayPeriod],
        ),
        (
            TimeGranularity::Minute,
            vec![
                DateSegmentKind::Hour,
                DateSegmentKind::Minute,
                DateSegmentKind::DayPeriod,
            ],
        ),
        (
            TimeGranularity::Second,
            vec![
                DateSegmentKind::Hour,
                DateSegmentKind::Minute,
                DateSegmentKind::Second,
                DateSegmentKind::DayPeriod,
            ],
        ),
    ] {
        let service = Service::<Machine>::new(
            props()
                .granularity(granularity)
                .hour_cycle(Some(HourCycle::H12)),
            &Env::default(),
            &Messages::default(),
        );

        let actual = service
            .context()
            .segments
            .iter()
            .filter(|segment| segment.is_editable)
            .map(|segment| segment.kind)
            .collect::<Vec<_>>();

        assert_eq!(actual, expected);
    }
}

#[test]
fn hidden_input_renders_iso_time_and_name() {
    let empty = service();

    let empty_api = empty.connect(&|_| {});

    assert_eq!(
        attr(&empty_api.hidden_input_attrs(), HtmlAttr::Value),
        Some("")
    );

    let filled = Service::<Machine>::new(
        props()
            .default_value(Some(time(14, 30, 45)))
            .name(Some(String::from("starts_at"))),
        &Env::default(),
        &Messages::default(),
    );

    let filled_api = filled.connect(&|_| {});

    let attrs = filled_api.hidden_input_attrs();

    assert_eq!(attr(&attrs, HtmlAttr::Value), Some("14:30:45"));
    assert_eq!(attr(&attrs, HtmlAttr::Name), Some("starts_at"));
}

#[test]
fn readonly_blocks_editing_and_disabled_blocks_user_focus() {
    let mut readonly = Service::<Machine>::new(
        props().readonly(true),
        &Env::default(),
        &Messages::default(),
    );

    drop(readonly.send(Event::FocusSegment {
        kind: DateSegmentKind::Hour,
    }));

    assert_eq!(readonly.state(), &State::Focused(DateSegmentKind::Hour));

    drop(readonly.send(Event::IncrementSegment {
        kind: DateSegmentKind::Hour,
    }));

    assert_eq!(
        readonly.context().get_segment_value(DateSegmentKind::Hour),
        None
    );

    let mut disabled = Service::<Machine>::new(
        props().disabled(true),
        &Env::default(),
        &Messages::default(),
    );

    drop(disabled.send(Event::FocusSegment {
        kind: DateSegmentKind::Hour,
    }));

    assert_eq!(disabled.state(), &State::Idle);

    drop(disabled.send(Event::SetValue(Some(time(10, 15, 0)))));

    assert_eq!(disabled.context().value.get(), &Some(time(10, 15, 0)));
}

#[test]
fn prop_sync_updates_context_backed_contract_without_replacing_ids() {
    let mut service = Service::<Machine>::new(props(), &Env::default(), &Messages::default());

    drop(
        service.set_props(
            props()
                .value(Some(time(14, 30, 0)))
                .granularity(TimeGranularity::Second)
                .hour_cycle(Some(HourCycle::H23))
                .disabled(true)
                .readonly(true)
                .force_leading_zeros(true),
        ),
    );

    let ctx = service.context();

    assert_eq!(ctx.ids.id(), "meeting-time");
    assert_eq!(ctx.value.get(), &Some(time(14, 30, 0)));
    assert_eq!(ctx.granularity, TimeGranularity::Second);
    assert_eq!(ctx.hour_cycle, HourCycle::H23);
    assert!(ctx.disabled);
    assert!(ctx.readonly);
    assert!(ctx.force_leading_zeros);
    assert!(
        !ctx.segments
            .iter()
            .any(|segment| segment.kind == DateSegmentKind::DayPeriod)
    );
}

#[test]
fn controlled_prop_sync_can_clear_value_without_submitting_stale_time() {
    let mut service = Service::<Machine>::new(
        props()
            .value(Some(time(14, 30, 0)))
            .name(Some(String::from("meeting_time"))),
        &Env::default(),
        &Messages::default(),
    );

    assert_eq!(service.context().value.get(), &Some(time(14, 30, 0)));
    assert_eq!(
        attr(
            &service.connect(&|_| {}).hidden_input_attrs(),
            HtmlAttr::Value
        ),
        Some("14:30:00")
    );

    drop(service.set_props(props().name(Some(String::from("meeting_time")))));

    assert!(service.context().value.is_controlled());
    assert_eq!(service.context().value.get(), &None);
    assert_eq!(
        attr(
            &service.connect(&|_| {}).hidden_input_attrs(),
            HtmlAttr::Value
        ),
        Some("")
    );
    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Hour),
        None
    );
}

#[test]
fn uncontrolled_prop_sync_clamps_current_value_to_new_bounds() {
    let mut service = Service::<Machine>::new(
        props()
            .hour_cycle(Some(HourCycle::H23))
            .default_value(Some(time(8, 30, 0)))
            .name(Some(String::from("meeting_time"))),
        &Env::default(),
        &Messages::default(),
    );

    drop(
        service.set_props(
            props()
                .hour_cycle(Some(HourCycle::H23))
                .min_value(Some(time(9, 0, 0)))
                .max_value(Some(time(17, 0, 0)))
                .name(Some(String::from("meeting_time"))),
        ),
    );

    assert!(!service.context().value.is_controlled());
    assert_eq!(service.context().value.get(), &Some(time(9, 0, 0)));
    assert_eq!(
        attr(
            &service.connect(&|_| {}).hidden_input_attrs(),
            HtmlAttr::Value
        ),
        Some("09:00:00")
    );
}

#[test]
fn min_and_max_values_clamp_published_time() {
    let mut service = Service::<Machine>::new(
        props()
            .hour_cycle(Some(HourCycle::H23))
            .min_value(Some(time(9, 0, 0)))
            .max_value(Some(time(17, 0, 0))),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FocusSegment {
        kind: DateSegmentKind::Hour,
    }));
    drop(service.send(Event::TypeIntoSegment {
        kind: DateSegmentKind::Hour,
        ch: '2',
    }));
    drop(service.send(Event::TypeIntoSegment {
        kind: DateSegmentKind::Hour,
        ch: '3',
    }));
    drop(service.send(Event::TypeIntoSegment {
        kind: DateSegmentKind::Minute,
        ch: '5',
    }));
    drop(service.send(Event::TypeIntoSegment {
        kind: DateSegmentKind::Minute,
        ch: '9',
    }));

    assert_eq!(service.context().value.get(), &Some(time(17, 0, 0)));
}

#[test]
fn part_attrs_dispatches_every_part() {
    let service = Service::<Machine>::new(
        props()
            .default_value(Some(time(14, 30, 45)))
            .granularity(TimeGranularity::Second)
            .hour_cycle(Some(HourCycle::H12)),
        &Env::default(),
        &Messages::default(),
    );

    let api = service.connect(&|_| {});

    for part in [
        Part::Root,
        Part::Label,
        Part::FieldGroup,
        Part::Segment {
            kind: DateSegmentKind::Hour,
        },
        Part::Literal { index: 0 },
        Part::Description,
        Part::ErrorMessage,
        Part::HiddenInput,
    ] {
        let debug = format!("{part:?}");

        assert!(
            !api.part_attrs(part).attrs().is_empty(),
            "part attrs should not be empty for {debug}"
        );
    }
}

#[test]
fn api_handlers_and_accessors_cover_direct_connect_surface() {
    let service = Service::<Machine>::new(
        props().default_value(Some(time(9, 45, 0))),
        &Env::default(),
        &Messages::default(),
    );

    let sent = RefCell::new(Vec::new());
    let send = |event| sent.borrow_mut().push(event);

    let api = Api::new(service.state(), service.context(), service.props(), &send);

    assert_eq!(api.segments().len(), service.context().segments.len());
    assert_eq!(api.value(), Some(&time(9, 45, 0)));
    assert!(!api.is_focused());

    api.on_segment_focus(DateSegmentKind::Hour);
    api.on_segment_click(DateSegmentKind::Minute);
    api.on_field_group_focusout(false);
    api.on_field_group_focusout(true);

    assert_eq!(
        sent.borrow().as_slice(),
        &[
            Event::FocusSegment {
                kind: DateSegmentKind::Hour,
            },
            Event::FocusSegment {
                kind: DateSegmentKind::Minute,
            },
            Event::BlurAll,
        ]
    );
}

#[test]
fn context_debug_and_partial_eq_ignore_backend_identity() {
    let service = service();

    let mut left = service.context().clone();
    let right = service.context().clone();

    assert!(format!("{left:?}").contains("Context"));
    assert_eq!(left, right);

    left.set_segment_value(DateSegmentKind::Hour, 3);

    assert_ne!(left, right);
}

#[test]
fn context_segment_helpers_preserve_completion_and_wrapping_contracts() {
    let mut ctx = Service::<Machine>::new(
        props()
            .granularity(TimeGranularity::Minute)
            .hour_cycle(Some(HourCycle::H23)),
        &Env::default(),
        &Messages::default(),
    )
    .context()
    .clone();

    assert!(!ctx.is_complete());

    ctx.set_segment_value(DateSegmentKind::Hour, 7);

    assert!(!ctx.is_complete());
    assert_eq!(ctx.assemble_time(), None);

    assert!(ctx.segment_mut(DateSegmentKind::Year).is_none());

    ctx.segment_mut(DateSegmentKind::Hour)
        .expect("hour segment exists")
        .value = Some(5);

    assert_eq!(ctx.get_segment_value(DateSegmentKind::Hour), Some(5));

    ctx.clear_segment_value(DateSegmentKind::Hour);

    let hour = ctx
        .segments
        .iter()
        .find(|segment| segment.kind == DateSegmentKind::Hour)
        .expect("hour segment exists");

    assert_eq!(hour.value, None);
    assert_eq!(hour.text, "");

    ctx.set_segment_value(DateSegmentKind::Hour, 0);
    ctx.decrement_segment(DateSegmentKind::Hour);

    assert_eq!(ctx.get_segment_value(DateSegmentKind::Hour), Some(23));

    ctx.increment_segment(DateSegmentKind::Hour);

    assert_eq!(ctx.get_segment_value(DateSegmentKind::Hour), Some(0));

    ctx.set_segment_value(DateSegmentKind::Hour, 5);
    ctx.increment_segment(DateSegmentKind::Hour);

    assert_eq!(ctx.get_segment_value(DateSegmentKind::Hour), Some(6));

    ctx.decrement_segment(DateSegmentKind::Hour);

    assert_eq!(ctx.get_segment_value(DateSegmentKind::Hour), Some(5));

    ctx.clear_segment_value(DateSegmentKind::Hour);
    ctx.decrement_segment(DateSegmentKind::Hour);

    assert_eq!(ctx.get_segment_value(DateSegmentKind::Hour), Some(23));

    ctx.clear_segment_value(DateSegmentKind::Minute);
    ctx.increment_segment(DateSegmentKind::Minute);

    assert_eq!(ctx.get_segment_value(DateSegmentKind::Minute), Some(0));

    let mut h12 = Service::<Machine>::new(
        props().hour_cycle(Some(HourCycle::H12)),
        &Env::default(),
        &Messages::default(),
    )
    .context()
    .clone();

    h12.set_segment_value(DateSegmentKind::Hour, 1);
    h12.decrement_segment(DateSegmentKind::Hour);

    assert_eq!(h12.get_segment_value(DateSegmentKind::Hour), Some(12));

    h12.increment_segment(DateSegmentKind::Hour);

    assert_eq!(h12.get_segment_value(DateSegmentKind::Hour), Some(1));

    h12.increment_segment(DateSegmentKind::DayPeriod);

    assert_eq!(h12.get_segment_value(DateSegmentKind::DayPeriod), Some(1));

    h12.decrement_segment(DateSegmentKind::DayPeriod);

    assert_eq!(h12.get_segment_value(DateSegmentKind::DayPeriod), Some(0));
}

#[test]
fn machine_guards_and_prop_sync_reconcile_focus() {
    let mut active = service();

    drop(active.send(Event::FocusSegment {
        kind: DateSegmentKind::Year,
    }));

    assert_eq!(active.state(), &State::Idle);

    drop(active.send(Event::FocusSegment {
        kind: DateSegmentKind::Hour,
    }));
    drop(active.set_props(props().readonly(true)));
    drop(active.send(Event::TypeIntoSegment {
        kind: DateSegmentKind::Hour,
        ch: '5',
    }));

    assert_eq!(
        active.context().get_segment_value(DateSegmentKind::Hour),
        None
    );

    drop(active.set_props(props().granularity(TimeGranularity::Hour)));
    drop(active.send(Event::FocusSegment {
        kind: DateSegmentKind::Minute,
    }));

    assert_ne!(active.state(), &State::Focused(DateSegmentKind::Minute));

    let mut minute_focus = Service::<Machine>::new(
        props().granularity(TimeGranularity::Minute),
        &Env::default(),
        &Messages::default(),
    );

    drop(minute_focus.send(Event::FocusSegment {
        kind: DateSegmentKind::Minute,
    }));
    drop(minute_focus.set_props(props().granularity(TimeGranularity::Hour)));

    assert_eq!(minute_focus.state(), &State::Idle);

    let mut disabled = service();

    drop(disabled.send(Event::FocusSegment {
        kind: DateSegmentKind::Hour,
    }));
    drop(disabled.set_props(props().disabled(true)));

    assert_eq!(disabled.state(), &State::Idle);
    assert_eq!(disabled.context().focused_segment, None);
}

#[test]
fn describedby_wiring_requires_invalid_error_pair() {
    let service = Service::<Machine>::new(
        props().error_message(Some(String::from("Required"))),
        &Env::default(),
        &Messages::default(),
    );

    let api = service.connect(&|_| {});

    assert_eq!(
        attr(
            &api.field_group_attrs(),
            HtmlAttr::Aria(AriaAttr::DescribedBy)
        ),
        None
    );
    assert_eq!(
        attr(
            &api.segment_attrs(&DateSegmentKind::Hour),
            HtmlAttr::Aria(AriaAttr::DescribedBy)
        ),
        None
    );

    let invalid = Service::<Machine>::new(
        props()
            .invalid(true)
            .error_message(Some(String::from("Required"))),
        &Env::default(),
        &Messages::default(),
    );

    let invalid_api = invalid.connect(&|_| {});

    assert_eq!(
        attr(
            &invalid_api.field_group_attrs(),
            HtmlAttr::Aria(AriaAttr::DescribedBy)
        ),
        Some("meeting-time-error-message")
    );
    assert_eq!(
        attr(
            &invalid_api.segment_attrs(&DateSegmentKind::Hour),
            HtmlAttr::Aria(AriaAttr::DescribedBy)
        ),
        Some("meeting-time-error-message")
    );
}

#[test]
fn keydown_dispatches_every_navigation_and_editing_event() {
    let service = service();

    let sent = RefCell::new(Vec::new());
    let send = |event| sent.borrow_mut().push(event);

    let api = service.connect(&send);

    let kind = DateSegmentKind::Hour;

    api.on_segment_keydown(
        kind,
        &key_data(KeyboardKey::ArrowUp, None),
        false,
        Direction::Ltr,
    );
    api.on_segment_keydown(
        kind,
        &key_data(KeyboardKey::ArrowDown, None),
        false,
        Direction::Ltr,
    );
    api.on_segment_keydown(
        kind,
        &key_data(KeyboardKey::ArrowLeft, None),
        false,
        Direction::Ltr,
    );
    api.on_segment_keydown(
        kind,
        &key_data(KeyboardKey::ArrowRight, None),
        false,
        Direction::Ltr,
    );
    api.on_segment_keydown(
        kind,
        &key_data(KeyboardKey::ArrowLeft, None),
        false,
        Direction::Rtl,
    );
    api.on_segment_keydown(
        kind,
        &key_data(KeyboardKey::ArrowRight, None),
        false,
        Direction::Rtl,
    );
    api.on_segment_keydown(
        kind,
        &key_data(KeyboardKey::Tab, None),
        true,
        Direction::Ltr,
    );
    api.on_segment_keydown(
        kind,
        &key_data(KeyboardKey::Tab, None),
        false,
        Direction::Ltr,
    );
    api.on_segment_keydown(
        kind,
        &key_data(KeyboardKey::Delete, None),
        false,
        Direction::Ltr,
    );
    api.on_segment_keydown(
        kind,
        &key_data(KeyboardKey::Escape, None),
        false,
        Direction::Ltr,
    );
    api.on_segment_keydown(
        kind,
        &key_data(KeyboardKey::Unidentified, Some('7')),
        false,
        Direction::Ltr,
    );

    assert_eq!(
        sent.borrow().as_slice(),
        &[
            Event::IncrementSegment { kind },
            Event::DecrementSegment { kind },
            Event::FocusPrevSegment,
            Event::FocusNextSegment,
            Event::FocusNextSegment,
            Event::FocusPrevSegment,
            Event::FocusPrevSegment,
            Event::FocusNextSegment,
            Event::ClearSegment { kind },
            Event::ClearAll,
            Event::TypeIntoSegment { kind, ch: '7' },
        ]
    );
}

#[test]
fn focused_api_and_typeahead_boundaries_are_observable() {
    let mut service = Service::<Machine>::new(
        props().hour_cycle(Some(HourCycle::H23)),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FocusSegment {
        kind: DateSegmentKind::Hour,
    }));

    let api = service.connect(&|_| {});

    assert!(api.is_focused());

    assert!(
        type_into_segment(
            service.context(),
            service.state(),
            DateSegmentKind::Year,
            '1',
        )
        .is_none()
    );
    assert!(
        type_into_segment(
            service.context(),
            service.state(),
            DateSegmentKind::Hour,
            'x',
        )
        .is_none()
    );

    drop(service.send(Event::TypeIntoSegment {
        kind: DateSegmentKind::Hour,
        ch: '3',
    }));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Hour),
        Some(3)
    );
    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Minute));

    let mut ctx = service.context().clone();

    ctx.focused_segment = Some(DateSegmentKind::Minute);
    ctx.type_buffer = String::from("7");

    commit_type_buffer(&mut ctx);

    assert_eq!(ctx.get_segment_value(DateSegmentKind::Minute), Some(7));
}

#[test]
fn time_conversion_helpers_cover_hour_cycles_cjk_clamp_and_digits() {
    assert_eq!(display_hour(time(13, 0, 0), HourCycle::H11), 1);
    assert_eq!(display_hour(time(23, 0, 0), HourCycle::H11), 11);
    assert_eq!(display_hour(time(0, 0, 0), HourCycle::H12), 12);
    assert_eq!(display_hour(time(12, 0, 0), HourCycle::H12), 12);
    assert_eq!(display_hour(time(0, 0, 0), HourCycle::H24), 24);

    assert_eq!(display_hour_to_24(0, Some(1), HourCycle::H11), Some(12));
    assert_eq!(display_hour_to_24(11, Some(1), HourCycle::H11), Some(23));
    assert_eq!(display_hour_to_24(1, Some(1), HourCycle::H12), Some(13));
    assert_eq!(display_hour_to_24(24, None, HourCycle::H24), Some(0));

    let mut ctx = Service::<Machine>::new(
        props()
            .granularity(TimeGranularity::Second)
            .hour_cycle(Some(HourCycle::H12)),
        &Env::default(),
        &Messages::default(),
    )
    .context()
    .clone();

    apply_segments_from_time(&mut ctx, time(13, 5, 9));

    assert_eq!(ctx.get_segment_value(DateSegmentKind::Hour), Some(1));
    assert_eq!(ctx.get_segment_value(DateSegmentKind::Minute), Some(5));
    assert_eq!(ctx.get_segment_value(DateSegmentKind::Second), Some(9));
    assert_eq!(ctx.get_segment_value(DateSegmentKind::DayPeriod), Some(1));

    let ko = Locale::parse("ko-KR").expect("ko-KR parses");

    assert_eq!(
        day_period_from_cjk_buffer("오전", &ko, HourCycle::H12, None, None),
        Some(0)
    );
    assert_eq!(
        day_period_from_cjk_buffer("오후", &ko, HourCycle::H12, None, None),
        Some(1)
    );

    let ja = Locale::parse("ja-JP").expect("ja-JP parses");

    assert_eq!(
        day_period_from_cjk_buffer("午前", &ja, HourCycle::H12, None, None),
        Some(0)
    );
    assert_eq!(
        day_period_from_cjk_buffer("午後", &ja, HourCycle::H12, None, None),
        Some(1)
    );

    assert_eq!(
        clamp_time(time(8, 0, 0), Some(&time(8, 0, 0)), Some(&time(17, 0, 0))),
        time(8, 0, 0)
    );
    assert_eq!(
        clamp_time(time(17, 0, 0), Some(&time(8, 0, 0)), Some(&time(17, 0, 0))),
        time(17, 0, 0)
    );
    assert_eq!(
        clamp_time(time(7, 59, 0), Some(&time(8, 0, 0)), Some(&time(17, 0, 0))),
        time(8, 0, 0)
    );
    assert_eq!(
        clamp_time(time(17, 1, 0), Some(&time(8, 0, 0)), Some(&time(17, 0, 0))),
        time(17, 0, 0)
    );

    assert_eq!(digits_needed(9), 1);
    assert_eq!(digits_needed(10), 2);
    assert_eq!(digits_needed(100), 3);
    assert_eq!(digits_needed(1_000), 4);
}

#[test]
fn snapshots_cover_time_field_attr_surface() {
    let filled_service = Service::<Machine>::new(
        props()
            .default_value(Some(time(14, 30, 45)))
            .granularity(TimeGranularity::Second)
            .hour_cycle(Some(HourCycle::H12))
            .required(true)
            .invalid(true)
            .description(Some(String::from("Pick a time")))
            .error_message(Some(String::from("Time is unavailable")))
            .name(Some(String::from("starts_at"))),
        &Env::default(),
        &Messages::default(),
    );

    let api = filled_service.connect(&|_| {});

    assert_snapshot!("root_idle", snapshot_attrs(&api.root_attrs()));
    assert_snapshot!("label", snapshot_attrs(&api.label_attrs()));
    assert_snapshot!(
        "field_group_required_invalid",
        snapshot_attrs(&api.field_group_attrs())
    );
    assert_snapshot!(
        "segment_hour_h12_set",
        snapshot_attrs(&api.segment_attrs(&DateSegmentKind::Hour))
    );
    assert_snapshot!(
        "segment_minute_set",
        snapshot_attrs(&api.segment_attrs(&DateSegmentKind::Minute))
    );
    assert_snapshot!(
        "segment_second_set",
        snapshot_attrs(&api.segment_attrs(&DateSegmentKind::Second))
    );
    assert_snapshot!(
        "segment_day_period_pm",
        snapshot_attrs(&api.segment_attrs(&DateSegmentKind::DayPeriod))
    );
    assert_snapshot!("literal", snapshot_attrs(&api.literal_attrs(0)));
    assert_snapshot!("description", snapshot_attrs(&api.description_attrs()));
    assert_snapshot!("error_message", snapshot_attrs(&api.error_message_attrs()));
    assert_snapshot!(
        "hidden_input_filled",
        snapshot_attrs(&api.hidden_input_attrs())
    );

    let empty = service();

    let empty_api = empty.connect(&|_| {});

    assert_snapshot!(
        "hidden_input_empty",
        snapshot_attrs(&empty_api.hidden_input_attrs())
    );

    let disabled = Service::<Machine>::new(
        props().disabled(true).readonly(true).invalid(true),
        &Env::default(),
        &Messages::default(),
    );

    let disabled_api = disabled.connect(&|_| {});

    assert_snapshot!(
        "root_disabled_readonly_invalid",
        snapshot_attrs(&disabled_api.root_attrs())
    );

    let h23 = Service::<Machine>::new(
        props()
            .default_value(Some(time(14, 30, 0)))
            .hour_cycle(Some(HourCycle::H23)),
        &Env::default(),
        &Messages::default(),
    );

    let h23_api = h23.connect(&|_| {});

    assert_snapshot!(
        "segment_hour_h23_set",
        snapshot_attrs(&h23_api.segment_attrs(&DateSegmentKind::Hour))
    );

    let placeholder = service();

    let placeholder_api = placeholder.connect(&|_| {});

    assert_snapshot!(
        "segment_hour_h12_placeholder",
        snapshot_attrs(&placeholder_api.segment_attrs(&DateSegmentKind::Hour))
    );

    let am = Service::<Machine>::new(
        props()
            .default_value(Some(time(2, 30, 0)))
            .hour_cycle(Some(HourCycle::H12)),
        &Env::default(),
        &Messages::default(),
    );

    let am_api = am.connect(&|_| {});

    assert_snapshot!(
        "segment_day_period_am",
        snapshot_attrs(&am_api.segment_attrs(&DateSegmentKind::DayPeriod))
    );

    let group = Service::<Machine>::new(props(), &Env::default(), &Messages::default());

    let group_api = group.connect(&|_| {});

    assert_snapshot!(
        "field_group_default",
        snapshot_attrs(&group_api.field_group_attrs())
    );
}
