use alloc::{string::String, sync::Arc, vec};
use core::cell::RefCell;

use ars_core::{
    AriaAttr, AttrMap, ConnectApi, Direction, Env, HtmlAttr, KeyboardKey, Service, StubIntlBackend,
};
use ars_i18n::{CalendarDate, CalendarSystem, Locale};
use ars_interactions::KeyboardEventData;
use insta::assert_snapshot;

use super::*;

fn date(year: i32, month: u8, day: u8) -> CalendarDate {
    CalendarDate::new_gregorian(year, month, day).expect("test date should be valid")
}

fn props() -> Props {
    Props::new().id("birthday").label("Birthday")
}

fn service() -> Service<Machine> {
    Service::<Machine>::new(props(), &Env::default(), &Messages::default())
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

fn composing_key_data(key: KeyboardKey) -> KeyboardEventData {
    KeyboardEventData {
        is_composing: true,
        ..key_data(key, None)
    }
}

fn attr(attrs: &AttrMap, name: HtmlAttr) -> Option<&str> {
    attrs.get(&name)
}

#[test]
fn props_builder_sets_every_field() {
    let value = date(2024, 2, 29);

    let default_value = date(2020, 1, 2);

    let min_value = date(2020, 1, 1);

    let max_value = date(2030, 12, 31);

    let order = vec![
        DateSegmentKind::Year,
        DateSegmentKind::Month,
        DateSegmentKind::Day,
    ];

    let props = Props::new()
        .id("date")
        .value(Some(value.clone()))
        .default_value(Some(default_value.clone()))
        .calendar(CalendarSystem::Iso8601)
        .granularity(DateGranularity::Day)
        .min_value(Some(min_value.clone()))
        .max_value(Some(max_value.clone()))
        .disabled(true)
        .readonly(true)
        .required(true)
        .auto_focus(true)
        .label("Date")
        .aria_label(Some(String::from("Birth date")))
        .aria_labelledby(Some(String::from("label-id")))
        .aria_describedby(Some(String::from("help-id")))
        .description(Some(String::from("Help")))
        .error_message(Some(String::from("Error")))
        .invalid(true)
        .name(Some(String::from("birth_date")))
        .segment_order(Some(order.clone()))
        .force_leading_zeros(true);

    assert_eq!(props.id, "date");
    assert_eq!(props.value, Some(Some(value)));
    assert_eq!(props.default_value, Some(default_value));
    assert_eq!(props.calendar, CalendarSystem::Iso8601);
    assert_eq!(props.granularity, DateGranularity::Day);
    assert_eq!(props.min_value, Some(min_value));
    assert_eq!(props.max_value, Some(max_value));
    assert!(props.disabled);
    assert!(props.readonly);
    assert!(props.required);
    assert!(props.auto_focus);
    assert_eq!(props.label, "Date");
    assert_eq!(props.aria_label.as_deref(), Some("Birth date"));
    assert_eq!(props.aria_labelledby.as_deref(), Some("label-id"));
    assert_eq!(props.aria_describedby.as_deref(), Some("help-id"));
    assert_eq!(props.description.as_deref(), Some("Help"));
    assert_eq!(props.error_message.as_deref(), Some("Error"));
    assert!(props.invalid);
    assert_eq!(props.name.as_deref(), Some("birth_date"));
    assert_eq!(props.segment_order, Some(order));
    assert!(props.force_leading_zeros);
}

#[test]
fn arrow_navigation_moves_between_segments() {
    let mut service = service();

    drop(service.send(Event::FocusSegment(DateSegmentKind::Month)));

    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Month));

    drop(service.send(Event::FocusNextSegment));

    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Day));

    drop(service.send(Event::FocusNextSegment));

    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Year));

    drop(service.send(Event::FocusPrevSegment));

    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Day));
}

#[test]
fn keydown_respects_ltr_and_rtl_arrow_direction() {
    let service = service();

    let sent = RefCell::new(Vec::new());
    let send = |event| sent.borrow_mut().push(event);

    let api = service.connect(&send);

    api.on_segment_keydown(
        DateSegmentKind::Month,
        &key_data(KeyboardKey::ArrowRight, None),
        false,
        Direction::Ltr,
    );

    api.on_segment_keydown(
        DateSegmentKind::Month,
        &key_data(KeyboardKey::ArrowRight, None),
        false,
        Direction::Rtl,
    );

    let sent = sent.borrow();

    assert_eq!(sent[0], Event::FocusNextSegment);
    assert_eq!(sent[1], Event::FocusPrevSegment);
}

#[test]
fn keydown_dispatches_segment_editing_commands() {
    let service = service();

    let sent = RefCell::new(Vec::new());
    let send = |event| sent.borrow_mut().push(event);

    let api = service.connect(&send);

    for (key, shift, expected) in [
        (
            KeyboardKey::ArrowUp,
            false,
            Event::IncrementSegment(DateSegmentKind::Month),
        ),
        (
            KeyboardKey::ArrowDown,
            false,
            Event::DecrementSegment(DateSegmentKind::Month),
        ),
        (KeyboardKey::ArrowLeft, false, Event::FocusPrevSegment),
        (KeyboardKey::Tab, true, Event::FocusPrevSegment),
        (KeyboardKey::Tab, false, Event::FocusNextSegment),
        (
            KeyboardKey::Backspace,
            false,
            Event::ClearSegment(DateSegmentKind::Month),
        ),
        (
            KeyboardKey::Delete,
            false,
            Event::ClearSegment(DateSegmentKind::Month),
        ),
        (KeyboardKey::Escape, false, Event::ClearAll),
    ] {
        api.on_segment_keydown(
            DateSegmentKind::Month,
            &key_data(key, None),
            shift,
            Direction::Ltr,
        );

        assert_eq!(sent.borrow().last(), Some(&expected));
    }

    api.on_segment_keydown(
        DateSegmentKind::Month,
        &key_data(KeyboardKey::Unidentified, Some('7')),
        false,
        Direction::Ltr,
    );

    assert_eq!(
        sent.borrow().last(),
        Some(&Event::TypeIntoSegment(DateSegmentKind::Month, '7'))
    );

    let len = sent.borrow().len();

    api.on_segment_keydown(
        DateSegmentKind::Month,
        &key_data(KeyboardKey::Home, None),
        false,
        Direction::Ltr,
    );

    assert_eq!(sent.borrow().len(), len);

    api.on_segment_keydown(
        DateSegmentKind::Month,
        &composing_key_data(KeyboardKey::Unidentified),
        false,
        Direction::Ltr,
    );

    assert_eq!(sent.borrow().last(), Some(&Event::CompositionStart));
}

#[test]
fn segment_handlers_emit_focus_composition_and_blur_events() {
    let service = service();

    let sent = RefCell::new(Vec::new());
    let send = |event| sent.borrow_mut().push(event);

    let api = service.connect(&send);

    api.on_field_group_focusout(false);

    assert!(sent.borrow().is_empty());

    api.on_field_group_focusout(true);
    api.on_segment_focus(DateSegmentKind::Day);
    api.on_segment_click(DateSegmentKind::Year);
    api.on_segment_composition_start();
    api.on_segment_composition_end(DateSegmentKind::Month, "12");

    assert_eq!(
        sent.into_inner(),
        vec![
            Event::BlurAll,
            Event::FocusSegment(DateSegmentKind::Day),
            Event::FocusSegment(DateSegmentKind::Year),
            Event::CompositionStart,
            Event::CompositionEnd(DateSegmentKind::Month, String::from("12")),
        ]
    );
}

#[test]
fn segment_editing_with_digits_fills_and_advances() {
    let mut service = service();

    drop(service.send(Event::FocusSegment(DateSegmentKind::Month)));
    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Month, '1')));
    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Month, '2')));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Month),
        Some(12)
    );
    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Day));
}

#[test]
fn year_waits_for_four_digits() {
    let mut service = service();

    drop(service.send(Event::FocusSegment(DateSegmentKind::Year)));

    for ch in ['2', '0', '2'] {
        drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Year, ch)));
    }

    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Year));

    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Year, '4')));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Year),
        Some(2024)
    );
}

#[test]
fn type_buffer_commit_publishes_numeric_and_month_name_buffers() {
    let mut service = service();

    drop(service.send(Event::FocusSegment(DateSegmentKind::Month)));
    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Month, '1')));

    assert_eq!(service.context().type_buffer, "1");

    drop(service.send(Event::TypeBufferCommit(DateSegmentKind::Month)));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Month),
        Some(1)
    );
    assert!(service.context().type_buffer.is_empty());

    drop(service.send(Event::FocusSegment(DateSegmentKind::Month)));
    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Month, 'J')));
    drop(service.send(Event::TypeBufferCommit(DateSegmentKind::Month)));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Month),
        Some(1)
    );
    assert!(service.context().type_buffer.is_empty());
}

#[test]
fn invalid_typeahead_characters_and_unfocused_typing_are_ignored() {
    let mut service = service();

    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Month, '1')));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Month),
        None
    );

    drop(service.send(Event::FocusSegment(DateSegmentKind::Day)));
    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Day, 'x')));
    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Literal, '1')));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Day),
        None
    );
    assert!(service.context().type_buffer.is_empty());
}

#[test]
fn incomplete_increment_uses_segment_range_wrapping() {
    let mut service = service();

    drop(service.send(Event::FocusSegment(DateSegmentKind::Day)));
    drop(service.send(Event::IncrementSegment(DateSegmentKind::Day)));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Day),
        Some(1)
    );

    drop(service.send(Event::DecrementSegment(DateSegmentKind::Day)));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Day),
        Some(31)
    );
}

#[test]
fn up_down_increment_and_decrement_segments() {
    let mut service = Service::<Machine>::new(
        props().default_value(Some(date(2024, 2, 28))),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::IncrementSegment(DateSegmentKind::Day)));

    assert_eq!(service.context().value.get(), &Some(date(2024, 2, 29)));

    drop(service.send(Event::DecrementSegment(DateSegmentKind::Day)));

    assert_eq!(service.context().value.get(), &Some(date(2024, 2, 28)));
}

#[test]
fn calendar_aware_bounds_track_february_leap_years() {
    let mut service = service();

    for (kind, digits) in [
        (DateSegmentKind::Year, "2024"),
        (DateSegmentKind::Month, "02"),
    ] {
        drop(service.send(Event::FocusSegment(kind)));

        for ch in digits.chars() {
            drop(service.send(Event::TypeIntoSegment(kind, ch)));
        }
    }

    let day = service
        .context()
        .segments
        .iter()
        .find(|segment| segment.kind == DateSegmentKind::Day)
        .expect("day segment exists");

    assert_eq!(day.max, 29);

    drop(service.send(Event::FocusSegment(DateSegmentKind::Year)));

    for ch in "2023".chars() {
        drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Year, ch)));
    }

    let day = service
        .context()
        .segments
        .iter()
        .find(|segment| segment.kind == DateSegmentKind::Day)
        .expect("day segment exists");

    assert_eq!(day.max, 28);
}

#[test]
fn min_max_constraints_clamp_published_value() {
    let mut service = Service::<Machine>::new(
        props()
            .min_value(Some(date(2024, 1, 10)))
            .max_value(Some(date(2024, 1, 20))),
        &Env::default(),
        &Messages::default(),
    );

    for (kind, digits) in [
        (DateSegmentKind::Month, "01"),
        (DateSegmentKind::Day, "01"),
        (DateSegmentKind::Year, "2024"),
    ] {
        drop(service.send(Event::FocusSegment(kind)));

        for ch in digits.chars() {
            drop(service.send(Event::TypeIntoSegment(kind, ch)));
        }
    }

    assert_eq!(service.context().value.get(), &Some(date(2024, 1, 10)));
    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Day),
        Some(10)
    );
}

#[test]
fn max_constraint_clamps_published_value() {
    let mut service = Service::<Machine>::new(
        props().max_value(Some(date(2024, 1, 20))),
        &Env::default(),
        &Messages::default(),
    );

    for (kind, digits) in [
        (DateSegmentKind::Month, "01"),
        (DateSegmentKind::Day, "31"),
        (DateSegmentKind::Year, "2024"),
    ] {
        drop(service.send(Event::FocusSegment(kind)));

        for ch in digits.chars() {
            drop(service.send(Event::TypeIntoSegment(kind, ch)));
        }
    }

    assert_eq!(service.context().value.get(), &Some(date(2024, 1, 20)));
    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Day),
        Some(20)
    );
}

#[test]
fn set_props_syncs_all_context_backed_props() {
    let mut service = Service::<Machine>::new(
        props().default_value(Some(date(2024, 1, 15))),
        &Env::default(),
        &Messages::default(),
    );

    let result = service.set_props(
        props()
            .default_value(Some(date(2024, 1, 15)))
            .disabled(true)
            .readonly(true)
            .invalid(true)
            .force_leading_zeros(true)
            .min_value(Some(date(2024, 1, 20)))
            .segment_order(Some(vec![
                DateSegmentKind::Year,
                DateSegmentKind::Month,
                DateSegmentKind::Day,
            ])),
    );

    assert!(result.state_changed || result.context_changed);
    assert!(service.context().disabled);
    assert!(service.context().readonly);
    assert!(service.context().invalid);
    assert!(service.context().force_leading_zeros);
    assert_eq!(service.context().value.get(), &Some(date(2024, 1, 20)));
    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Day),
        Some(20)
    );

    let kinds = service
        .context()
        .segments
        .iter()
        .filter(|segment| segment.is_editable)
        .map(|segment| segment.kind)
        .collect::<Vec<_>>();

    assert_eq!(
        kinds,
        vec![
            DateSegmentKind::Year,
            DateSegmentKind::Month,
            DateSegmentKind::Day
        ]
    );
}

#[test]
fn set_props_defers_controlled_value_while_type_buffer_is_active() {
    let mut service = Service::<Machine>::new(
        props().value(Some(date(2024, 5, 1))),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FocusSegment(DateSegmentKind::Month)));
    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Month, '1')));
    drop(service.set_props(props().value(Some(date(2024, 12, 25)))));

    assert_eq!(service.context().value.get(), &Some(date(2024, 5, 1)));
    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Month),
        Some(1)
    );
    assert_eq!(service.context().type_buffer, "1");
    assert_eq!(
        service.context().pending_controlled_value,
        Some(Some(date(2024, 12, 25)))
    );

    drop(service.send(Event::BlurAll));

    assert_eq!(service.context().value.get(), &Some(date(2024, 12, 25)));
    assert!(service.context().pending_controlled_value.is_none());
}

#[test]
fn sync_props_refocuses_when_segment_set_removes_focused_segment() {
    let mut service = Service::<Machine>::new(
        props().calendar(CalendarSystem::Japanese),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FocusSegment(DateSegmentKind::Era)));
    drop(service.set_props(props().calendar(CalendarSystem::Gregorian)));

    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Month));
    assert_eq!(
        service.context().focused_segment,
        Some(DateSegmentKind::Month)
    );
    assert!(
        service
            .context()
            .segments
            .iter()
            .all(|segment| segment.kind != DateSegmentKind::Era)
    );
}

#[test]
fn disabled_state_allows_props_to_resync_context() {
    let mut service = Service::<Machine>::new(
        props().default_value(Some(date(2024, 1, 1))).disabled(true),
        &Env::default(),
        &Messages::default(),
    );

    drop(
        service.set_props(
            props()
                .default_value(Some(date(2024, 1, 1)))
                .value(Some(date(2024, 5, 6)))
                .disabled(true)
                .invalid(true),
        ),
    );

    assert_eq!(service.context().value.get(), &Some(date(2024, 5, 6)));
    assert!(service.context().disabled);
    assert!(service.context().invalid);
}

#[test]
fn controlled_empty_value_is_distinct_from_uncontrolled_default() {
    let service = Service::<Machine>::new(
        props().value(None).default_value(Some(date(2024, 1, 1))),
        &Env::default(),
        &Messages::default(),
    );

    assert!(service.context().value.is_controlled());
    assert_eq!(service.context().value.get(), &None);
}

#[test]
fn set_props_can_clear_a_controlled_value_without_releasing_control() {
    let mut service = Service::<Machine>::new(
        props().value(Some(date(2024, 1, 1))),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.set_props(props().value(None)));

    assert!(service.context().value.is_controlled());
    assert_eq!(service.context().value.get(), &None);
}

#[test]
fn auto_focus_initializes_first_editable_segment_as_focused() {
    let service = Service::<Machine>::new(
        props().auto_focus(true),
        &Env::default(),
        &Messages::default(),
    );

    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Month));
    assert_eq!(
        service.context().focused_segment,
        Some(DateSegmentKind::Month)
    );
}

#[test]
fn numeric_typeahead_returns_timer_marker_effect_until_value_is_complete() {
    let mut service = service();

    drop(service.send(Event::FocusSegment(DateSegmentKind::Month)));

    let result = service.send(Event::TypeIntoSegment(DateSegmentKind::Month, '1'));

    assert_eq!(service.context().type_buffer, "1");
    assert_eq!(result.pending_effects.len(), 1);
    assert_eq!(result.pending_effects[0].name, "type-buffer-commit");
}

#[test]
fn month_name_typeahead_matches_locale_month_prefixes() {
    let mut service = service();

    drop(service.send(Event::FocusSegment(DateSegmentKind::Month)));
    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Month, 'J')));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Month),
        Some(1)
    );

    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Month, 'u')));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Month),
        Some(6)
    );
    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Month));

    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Month, 'l')));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Month),
        Some(7)
    );
    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Day));
}

#[test]
fn locale_calendar_extension_selects_calendar_when_prop_is_default() {
    let env = Env {
        locale: Locale::parse("th-TH-u-ca-buddhist").expect("valid locale"),
        intl_backend: Arc::new(StubIntlBackend),
    };

    let service = Service::<Machine>::new(props(), &env, &Messages::default());

    assert_eq!(service.context().calendar, CalendarSystem::Buddhist);
}

#[test]
fn japanese_calendar_uses_era_segment_and_ideographic_literals() {
    let service = Service::<Machine>::new(
        props().calendar(CalendarSystem::Japanese),
        &Env {
            locale: Locale::parse("ja-JP").expect("valid locale"),
            intl_backend: Arc::new(StubIntlBackend),
        },
        &Messages::default(),
    );

    let rendered = service
        .context()
        .segments
        .iter()
        .map(|segment| {
            segment
                .literal
                .clone()
                .unwrap_or_else(|| segment.kind.data_name().to_string())
        })
        .collect::<Vec<_>>();

    assert_eq!(
        rendered,
        vec!["era", "year", "年", "month", "月", "day", "日"]
    );
}

#[test]
fn hidden_input_renders_iso_form_value() {
    let service = Service::<Machine>::new(
        props()
            .default_value(Some(date(2024, 2, 29)))
            .name(Some(String::from("birthday"))),
        &Env::default(),
        &Messages::default(),
    );

    let api = service.connect(&|_| {});

    let attrs = api.hidden_input_attrs();

    assert_eq!(attr(&attrs, HtmlAttr::Type), Some("hidden"));
    assert_eq!(attr(&attrs, HtmlAttr::Name), Some("birthday"));
    assert_eq!(attr(&attrs, HtmlAttr::Value), Some("2024-02-29"));
}

#[test]
fn connect_api_sets_group_and_segment_aria() {
    let service = Service::<Machine>::new(
        props()
            .default_value(Some(date(2024, 3, 15)))
            .required(true),
        &Env::default(),
        &Messages::default(),
    );

    let api = service.connect(&|_| {});

    let group = api.field_group_attrs();

    assert_eq!(attr(&group, HtmlAttr::Role), Some("group"));
    assert_eq!(
        attr(&group, HtmlAttr::Aria(AriaAttr::Required)),
        Some("true")
    );

    let month = api.segment_attrs(&DateSegmentKind::Month);

    assert_eq!(attr(&month, HtmlAttr::Role), Some("spinbutton"));
    assert_eq!(attr(&month, HtmlAttr::Aria(AriaAttr::ValueMin)), Some("1"));
    assert_eq!(attr(&month, HtmlAttr::Aria(AriaAttr::ValueMax)), Some("12"));
    assert_eq!(attr(&month, HtmlAttr::Aria(AriaAttr::ValueNow)), Some("3"));
    assert_eq!(
        attr(&month, HtmlAttr::Aria(AriaAttr::ValueText)),
        Some("March")
    );
    assert_eq!(attr(&month, HtmlAttr::Aria(AriaAttr::Label)), Some("Month"));
}

#[test]
fn placeholder_segments_use_placeholder_aria_value_text() {
    let service = service();

    let api = service.connect(&|_| {});

    let month = api.segment_attrs(&DateSegmentKind::Month);

    assert_eq!(
        attr(&month, HtmlAttr::Aria(AriaAttr::ValueText)),
        Some("mm")
    );
    assert_eq!(month.get_value(&HtmlAttr::Aria(AriaAttr::ValueNow)), None);
}

#[test]
fn disabled_ignores_user_events_but_accepts_set_value() {
    let mut service = Service::<Machine>::new(
        props().disabled(true),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FocusSegment(DateSegmentKind::Month)));

    assert_eq!(service.state(), &State::Idle);

    drop(service.send(Event::SetValue(Some(date(2024, 4, 5)))));

    assert_eq!(service.context().value.get(), &Some(date(2024, 4, 5)));
}

#[test]
fn clear_segment_and_clear_all_reset_values_and_focus() {
    let mut service = Service::<Machine>::new(
        props().default_value(Some(date(2024, 4, 5))),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FocusSegment(DateSegmentKind::Month)));
    drop(service.send(Event::ClearSegment(DateSegmentKind::Month)));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Month),
        None
    );
    assert_eq!(service.context().value.get(), &None);
    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Month));

    let mut service = Service::<Machine>::new(
        props().default_value(Some(date(2024, 4, 5))),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FocusSegment(DateSegmentKind::Day)));
    drop(service.send(Event::ClearAll));

    assert_eq!(service.state(), &State::Idle);
    assert_eq!(service.context().value.get(), &None);
    assert!(service.context().focused_segment.is_none());

    for kind in [
        DateSegmentKind::Month,
        DateSegmentKind::Day,
        DateSegmentKind::Year,
    ] {
        assert_eq!(service.context().get_segment_value(kind), None);
    }
}

#[test]
fn non_editable_focus_and_prev_from_first_segment_are_ignored() {
    let mut service = service();

    drop(service.send(Event::FocusSegment(DateSegmentKind::Literal)));

    assert_eq!(service.state(), &State::Idle);

    drop(service.send(Event::FocusSegment(DateSegmentKind::Month)));
    drop(service.send(Event::FocusPrevSegment));

    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Month));
}

#[test]
fn readonly_allows_focus_but_blocks_edits() {
    let mut service = Service::<Machine>::new(
        props().readonly(true),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FocusSegment(DateSegmentKind::Month)));
    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Month, '1')));

    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Month));
    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Month),
        None
    );
}

#[test]
fn set_value_rebuilds_segments() {
    let mut service = service();

    drop(service.send(Event::SetValue(Some(date(2024, 6, 7)))));

    assert_eq!(service.context().value.get(), &Some(date(2024, 6, 7)));
    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Month),
        Some(6)
    );
}

#[test]
fn active_buffer_defers_controlled_update_until_blur() {
    let mut service = Service::<Machine>::new(
        props().value(Some(date(2024, 1, 1))),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FocusSegment(DateSegmentKind::Month)));
    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Month, '1')));
    drop(service.send(Event::SetValue(Some(date(2024, 12, 25)))));

    assert!(service.context().pending_controlled_value.is_some());

    drop(service.send(Event::BlurAll));

    assert_eq!(service.context().value.get(), &Some(date(2024, 12, 25)));
    assert!(service.context().pending_controlled_value.is_none());
}

#[test]
fn ime_composition_suppresses_character_typing_until_end() {
    let mut service = service();

    drop(service.send(Event::FocusSegment(DateSegmentKind::Month)));
    drop(service.send(Event::CompositionStart));
    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::Month, '1')));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Month),
        None
    );

    drop(service.send(Event::CompositionEnd(
        DateSegmentKind::Month,
        String::from("12"),
    )));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Month),
        Some(12)
    );
}

#[test]
fn locale_order_can_be_overridden() {
    let service = Service::<Machine>::new(
        props().segment_order(Some(vec![
            DateSegmentKind::Year,
            DateSegmentKind::Month,
            DateSegmentKind::Day,
        ])),
        &Env::default(),
        &Messages::default(),
    );

    let kinds = service
        .context()
        .segments
        .iter()
        .filter(|segment| segment.is_editable)
        .map(|segment| segment.kind)
        .collect::<Vec<_>>();

    assert_eq!(
        kinds,
        vec![
            DateSegmentKind::Year,
            DateSegmentKind::Month,
            DateSegmentKind::Day
        ]
    );
}

#[test]
fn locale_order_uses_environment_locale() {
    let env = Env {
        locale: Locale::parse("de-DE").expect("valid locale"),
        intl_backend: Arc::new(StubIntlBackend),
    };

    let service = Service::<Machine>::new(props(), &env, &Messages::default());

    let kinds = service
        .context()
        .segments
        .iter()
        .filter(|segment| segment.is_editable)
        .map(|segment| segment.kind)
        .collect::<Vec<_>>();

    assert_eq!(
        kinds,
        vec![
            DateSegmentKind::Day,
            DateSegmentKind::Month,
            DateSegmentKind::Year
        ]
    );
}

#[test]
fn locale_specific_orders_and_literals_are_resolved() {
    for (locale, expected_kinds, expected_literal) in [
        (
            "zh-CN",
            vec![
                DateSegmentKind::Year,
                DateSegmentKind::Month,
                DateSegmentKind::Day,
            ],
            "/",
        ),
        (
            "ko-KR",
            vec![
                DateSegmentKind::Year,
                DateSegmentKind::Month,
                DateSegmentKind::Day,
            ],
            ". ",
        ),
    ] {
        let env = Env {
            locale: Locale::parse(locale).expect("valid locale"),
            intl_backend: Arc::new(StubIntlBackend),
        };

        let service = Service::<Machine>::new(props(), &env, &Messages::default());

        let kinds = service
            .context()
            .segments
            .iter()
            .filter(|segment| segment.is_editable)
            .map(|segment| segment.kind)
            .collect::<Vec<_>>();

        let literal = service
            .context()
            .segments
            .iter()
            .find_map(|segment| segment.literal.as_deref())
            .expect("literal segment exists");

        assert_eq!(kinds, expected_kinds);
        assert_eq!(literal, expected_literal);
    }
}

#[test]
fn explicit_calendar_prop_overrides_locale_calendar_extension() {
    let env = Env {
        locale: Locale::parse("th-TH-u-ca-buddhist").expect("valid locale"),
        intl_backend: Arc::new(StubIntlBackend),
    };

    let service = Service::<Machine>::new(
        props().calendar(CalendarSystem::Iso8601),
        &env,
        &Messages::default(),
    );

    assert_eq!(service.context().calendar, CalendarSystem::Iso8601);
}

#[test]
fn connect_api_helpers_and_part_attrs_cover_all_parts() {
    let mut service = Service::<Machine>::new(
        props()
            .default_value(Some(date(2024, 3, 15)))
            .description(Some(String::from("Help")))
            .error_message(Some(String::from("Error")))
            .invalid(true)
            .name(Some(String::from("birthday"))),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FocusSegment(DateSegmentKind::Month)));

    let api = service.connect(&|_| {});

    assert!(api.is_focused());
    assert_eq!(api.value(), Some(&date(2024, 3, 15)));
    assert_eq!(api.segments().len(), service.context().segments.len());

    for part in [
        Part::Root,
        Part::Label,
        Part::FieldGroup,
        Part::Segment {
            kind: DateSegmentKind::Month,
        },
        Part::Literal { index: 1 },
        Part::Description,
        Part::ErrorMessage,
        Part::HiddenInput,
    ] {
        let attrs = api.part_attrs(part);

        assert_eq!(
            attr(&attrs, HtmlAttr::Data("ars-scope")),
            Some("date-field")
        );
    }

    let missing_segment = api.segment_attrs(&DateSegmentKind::TimeZoneName);

    assert_eq!(attr(&missing_segment, HtmlAttr::Data("ars-scope")), None);
    assert_eq!(
        api.segment_attrs(&DateSegmentKind::Literal),
        api.literal_attrs(0)
    );
}

#[test]
fn aria_label_precedence_and_extra_descriptions_are_applied() {
    let service = Service::<Machine>::new(
        props()
            .aria_label(Some(String::from("Explicit label")))
            .aria_labelledby(Some(String::from("external-label")))
            .aria_describedby(Some(String::from("external-help")))
            .description(Some(String::from("Help"))),
        &Env::default(),
        &Messages::default(),
    );

    let group = service.connect(&|_| {}).field_group_attrs();

    assert_eq!(
        attr(&group, HtmlAttr::Aria(AriaAttr::Label)),
        Some("Explicit label")
    );
    assert_eq!(group.get_value(&HtmlAttr::Aria(AriaAttr::LabelledBy)), None);
    assert_eq!(
        attr(&group, HtmlAttr::Aria(AriaAttr::DescribedBy)),
        Some("birthday-description external-help")
    );
}

#[test]
fn date_segment_kind_helpers_cover_shared_segment_variants() {
    let messages = Messages::default();

    let locale = Locale::parse("en-US").expect("valid locale");

    for (kind, editable, numeric, label, name) in [
        (DateSegmentKind::Year, true, true, "Year", "year"),
        (DateSegmentKind::Month, true, true, "Month", "month"),
        (DateSegmentKind::Day, true, true, "Day", "day"),
        (DateSegmentKind::Hour, true, true, "Hour", "hour"),
        (DateSegmentKind::Minute, true, true, "Minute", "minute"),
        (DateSegmentKind::Second, true, true, "Second", "second"),
        (
            DateSegmentKind::DayPeriod,
            true,
            false,
            "AM/PM",
            "day-period",
        ),
        (
            DateSegmentKind::Weekday,
            false,
            false,
            "Day of week",
            "weekday",
        ),
        (DateSegmentKind::Era, true, false, "Era", "era"),
        (DateSegmentKind::Literal, false, false, "", "literal"),
        (
            DateSegmentKind::TimeZoneName,
            false,
            false,
            "Time zone",
            "time-zone-name",
        ),
    ] {
        assert_eq!(kind.is_editable(), editable, "{kind:?}");
        assert_eq!(kind.is_numeric(), numeric, "{kind:?}");
        assert_eq!(kind.aria_label(&messages, &locale), label);
        assert_eq!(kind.data_name(), name);
    }
}

#[test]
fn date_segment_display_and_day_period_aria_value_text_are_resolved() {
    let backend = StubIntlBackend;

    let locale = Locale::parse("en-US").expect("valid locale");

    let mut segment = DateSegment::new_numeric(DateSegmentKind::DayPeriod, 0, 1, "am/pm");

    assert_eq!(segment.display_text(), "am/pm");
    assert_eq!(segment.aria_value_text(&backend, &locale), None);

    segment.value = Some(1);
    segment.text = String::from("PM");

    assert_eq!(segment.display_text(), "PM");
    assert_eq!(
        segment.aria_value_text(&backend, &locale).as_deref(),
        Some("PM")
    );
}

#[test]
fn debug_impls_and_explicit_api_constructor_are_covered() {
    let props = props();

    let (state, ctx) =
        <Machine as ars_core::Machine>::init(&props, &Env::default(), &Messages::default());

    let api = Api::new(&state, &ctx, &props, &|_| {});

    let context_debug = format!("{ctx:?}");

    assert!(context_debug.contains("Context"));
    assert!(context_debug.contains("<dyn IntlBackend>"));

    let api_debug = format!("{api:?}");

    assert!(api_debug.contains("Api"));
    assert!(api_debug.contains("state"));
}

#[test]
fn props_changed_hook_emits_only_for_real_changes() {
    let original = props();

    assert!(<Machine as ars_core::Machine>::on_props_changed(&original, &original).is_empty());
    assert_eq!(
        <Machine as ars_core::Machine>::on_props_changed(
            &original,
            &original.clone().invalid(true)
        ),
        vec![Event::SyncProps(Box::new(original.clone().invalid(true)))]
    );
}

#[test]
#[should_panic(expected = "date_field::Props.id must remain stable after init")]
fn props_changed_hook_panics_when_id_changes() {
    drop(<Machine as ars_core::Machine>::on_props_changed(
        &props().id("before"),
        &props().id("after"),
    ));
}

#[test]
fn focus_next_from_idle_moves_to_first_segment() {
    let mut service = service();

    drop(service.send(Event::FocusNextSegment));

    assert_eq!(service.state(), &State::Focused(DateSegmentKind::Month));
    assert_eq!(
        service.context().focused_segment,
        Some(DateSegmentKind::Month)
    );
}

#[test]
fn readonly_blocks_all_editing_events() {
    let mut service = Service::<Machine>::new(
        props().readonly(true).default_value(Some(date(2024, 1, 2))),
        &Env::default(),
        &Messages::default(),
    );

    let before = service.context().clone();

    for event in [
        Event::IncrementSegment(DateSegmentKind::Day),
        Event::DecrementSegment(DateSegmentKind::Day),
        Event::TypeBufferCommit(DateSegmentKind::Day),
        Event::CompositionEnd(DateSegmentKind::Month, String::from("12")),
        Event::ClearSegment(DateSegmentKind::Day),
        Event::ClearAll,
    ] {
        let result = service.send(event);

        assert!(!result.state_changed);
        assert!(!result.context_changed);
        assert_eq!(service.context().value.get(), before.value.get());
    }
}

#[test]
fn rtl_arrow_left_moves_to_next_segment() {
    let service = service();

    let sent = RefCell::new(Vec::new());
    let send = |event| sent.borrow_mut().push(event);

    let api = service.connect(&send);

    api.on_segment_keydown(
        DateSegmentKind::Month,
        &key_data(KeyboardKey::ArrowLeft, None),
        false,
        Direction::Rtl,
    );

    assert_eq!(sent.into_inner(), vec![Event::FocusNextSegment]);
}

#[test]
fn labelledby_is_used_when_no_explicit_aria_label_exists() {
    let service = Service::<Machine>::new(
        props().aria_labelledby(Some(String::from("external-label"))),
        &Env::default(),
        &Messages::default(),
    );

    let group = service.connect(&|_| {}).field_group_attrs();

    assert_eq!(
        attr(&group, HtmlAttr::Aria(AriaAttr::LabelledBy)),
        Some("external-label")
    );
    assert_eq!(group.get_value(&HtmlAttr::Aria(AriaAttr::Label)), None);
}

#[test]
fn shared_time_segment_ranges_and_day_period_typeahead_are_supported() {
    let mut service = Service::<Machine>::new(
        props()
            .segment_order(Some(vec![
                DateSegmentKind::DayPeriod,
                DateSegmentKind::Hour,
                DateSegmentKind::Minute,
                DateSegmentKind::Second,
                DateSegmentKind::Year,
                DateSegmentKind::Month,
                DateSegmentKind::Day,
            ]))
            .default_value(Some(date(2024, 1, 2))),
        &Env::default(),
        &Messages::default(),
    );

    assert_eq!(
        service.context().segment_range(DateSegmentKind::Hour),
        (0, 23)
    );
    assert_eq!(
        service.context().segment_range(DateSegmentKind::Minute),
        (0, 59)
    );
    assert_eq!(
        service.context().segment_range(DateSegmentKind::Second),
        (0, 59)
    );
    assert_eq!(
        service.context().segment_range(DateSegmentKind::DayPeriod),
        (0, 1)
    );

    drop(service.send(Event::FocusSegment(DateSegmentKind::DayPeriod)));
    drop(service.send(Event::TypeIntoSegment(DateSegmentKind::DayPeriod, 'p')));

    assert_eq!(
        service
            .context()
            .get_segment_value(DateSegmentKind::DayPeriod),
        Some(1)
    );

    drop(service.send(Event::IncrementSegment(DateSegmentKind::Hour)));
    drop(service.send(Event::DecrementSegment(DateSegmentKind::Minute)));

    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Hour),
        Some(0)
    );
    assert_eq!(
        service.context().get_segment_value(DateSegmentKind::Minute),
        Some(59)
    );
}

#[test]
fn japanese_era_value_text_uses_localized_era_names() {
    let mut service = Service::<Machine>::new(
        props().calendar(CalendarSystem::Japanese),
        &Env {
            locale: Locale::parse("ja-JP").expect("valid locale"),
            intl_backend: Arc::new(StubIntlBackend),
        },
        &Messages::default(),
    );

    service
        .context_mut()
        .set_segment_value(DateSegmentKind::Era, 1);

    let attrs = service
        .connect(&|_| {})
        .segment_attrs(&DateSegmentKind::Era);

    assert_ne!(attr(&attrs, HtmlAttr::Aria(AriaAttr::ValueText)), Some("1"));
}

#[test]
fn private_locale_and_format_helpers_cover_edge_branches() {
    let locale = Locale::parse("ja-JP").expect("valid locale");

    assert_eq!(
        segment_order_for_locale(&locale, CalendarSystem::Japanese),
        vec![
            DateSegmentKind::Era,
            DateSegmentKind::Year,
            DateSegmentKind::Month,
            DateSegmentKind::Day,
        ]
    );

    let backend = StubIntlBackend;

    assert_eq!(
        format_segment_value(&backend, &locale, DateSegmentKind::DayPeriod, 1, false),
        "PM"
    );
    assert_eq!(digits_needed(0), 1);
}

#[test]
fn date_field_connect_snapshots() {
    let service = Service::<Machine>::new(
        props()
            .default_value(Some(date(2024, 3, 15)))
            .description(Some(String::from("Use legal birth date")))
            .error_message(Some(String::from("Invalid date")))
            .invalid(true)
            .required(true)
            .name(Some(String::from("birthday"))),
        &Env::default(),
        &Messages::default(),
    );

    let api = service.connect(&|_| {});

    assert_snapshot!("root_invalid", snapshot_attrs(&api.root_attrs()));
    assert_snapshot!("label", snapshot_attrs(&api.label_attrs()));
    assert_snapshot!(
        "field_group_invalid",
        snapshot_attrs(&api.field_group_attrs())
    );
    assert_snapshot!(
        "segment_month_set",
        snapshot_attrs(&api.segment_attrs(&DateSegmentKind::Month))
    );
    assert_snapshot!("literal", snapshot_attrs(&api.literal_attrs(1)));
    assert_snapshot!("description", snapshot_attrs(&api.description_attrs()));
    assert_snapshot!("error_message", snapshot_attrs(&api.error_message_attrs()));
    assert_snapshot!(
        "hidden_input_filled",
        snapshot_attrs(&api.hidden_input_attrs())
    );
}

#[test]
fn date_field_disabled_readonly_snapshots() {
    let service = Service::<Machine>::new(
        props()
            .default_value(Some(date(2024, 3, 15)))
            .disabled(true)
            .readonly(true)
            .invalid(true)
            .error_message(Some(String::from("Invalid date"))),
        &Env::default(),
        &Messages::default(),
    );

    let api = service.connect(&|_| {});

    assert_snapshot!("root_disabled_readonly", snapshot_attrs(&api.root_attrs()));
    assert_snapshot!(
        "field_group_disabled_readonly",
        snapshot_attrs(&api.field_group_attrs())
    );
    assert_snapshot!(
        "segment_month_disabled_readonly",
        snapshot_attrs(&api.segment_attrs(&DateSegmentKind::Month))
    );
}

#[test]
fn date_field_explicit_aria_label_snapshots() {
    let service = Service::<Machine>::new(
        props()
            .aria_label(Some(String::from("Birth date")))
            .aria_labelledby(Some(String::from("external-label")))
            .aria_describedby(Some(String::from("external-help")))
            .description(Some(String::from("Help"))),
        &Env::default(),
        &Messages::default(),
    );

    let api = service.connect(&|_| {});

    assert_snapshot!(
        "field_group_explicit_aria_label",
        snapshot_attrs(&api.field_group_attrs())
    );
}

#[test]
fn date_field_japanese_calendar_snapshots() {
    let service = Service::<Machine>::new(
        props().calendar(CalendarSystem::Japanese),
        &Env {
            locale: Locale::parse("ja-JP").expect("valid locale"),
            intl_backend: Arc::new(StubIntlBackend),
        },
        &Messages::default(),
    );

    let api = service.connect(&|_| {});

    assert_snapshot!(
        "segment_era_placeholder",
        snapshot_attrs(&api.segment_attrs(&DateSegmentKind::Era))
    );
    assert_snapshot!("japanese_literal", snapshot_attrs(&api.literal_attrs(2)));
}

#[test]
fn date_field_placeholder_snapshots() {
    let service = service();

    let api = service.connect(&|_| {});

    assert_snapshot!("root_idle", snapshot_attrs(&api.root_attrs()));
    assert_snapshot!(
        "segment_month_placeholder",
        snapshot_attrs(&api.segment_attrs(&DateSegmentKind::Month))
    );
    assert_snapshot!(
        "hidden_input_empty",
        snapshot_attrs(&api.hidden_input_attrs())
    );
}
