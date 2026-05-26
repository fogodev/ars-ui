use ars_components::date_time::{
    calendar::{self, PageBehavior, SelectionMode},
    date_field::{self, DateSegmentKind},
    time_field,
};
use ars_core::{Env, KeyboardKey, Service};
use ars_i18n::{CalendarDate, HourCycle, Time, Weekday};
use proptest::prelude::*;

#[derive(Clone, Debug)]
enum DateFieldAction {
    Send(date_field::Event),
    SetControlledValue(Option<CalendarDate>),
    SetDisabled(bool),
    SetReadonly(bool),
    SetInvalid(bool),
}

#[derive(Clone, Debug)]
enum TimeFieldAction {
    Send(time_field::Event),
    SetControlledValue(Option<Time>),
    SetDisabled(bool),
    SetReadonly(bool),
    SetInvalid(bool),
}

fn date(year: i32, month: u8, day: u8) -> CalendarDate {
    CalendarDate::new_gregorian(year, month, day).expect("generated date should be valid")
}

fn date_field_props() -> date_field::Props {
    date_field::Props::new().id("date-field").label("Date")
}

fn time_field_props() -> time_field::Props {
    time_field::Props::new()
        .id("time-field")
        .label("Time")
        .hour_cycle(Some(HourCycle::H12))
        .granularity(time_field::TimeGranularity::Second)
}

fn arb_date() -> impl Strategy<Value = CalendarDate> {
    (1900i32..=2100, 1u8..=12, 1u8..=28).prop_map(|(year, month, day)| date(year, month, day))
}

fn arb_time() -> impl Strategy<Value = Time> {
    (0u8..=23, 0u8..=59, 0u8..=59)
        .prop_map(|(hour, minute, second)| Time::new(hour, minute, second, 0).unwrap())
}

fn arb_editable_kind() -> impl Strategy<Value = DateSegmentKind> {
    prop_oneof![
        Just(DateSegmentKind::Year),
        Just(DateSegmentKind::Month),
        Just(DateSegmentKind::Day),
    ]
}

fn arb_digit() -> impl Strategy<Value = char> {
    (0u8..=9).prop_map(|digit| char::from(b'0' + digit))
}

fn arb_time_editable_kind() -> impl Strategy<Value = DateSegmentKind> {
    prop_oneof![
        Just(DateSegmentKind::Hour),
        Just(DateSegmentKind::Minute),
        Just(DateSegmentKind::Second),
        Just(DateSegmentKind::DayPeriod),
    ]
}

fn arb_date_field_action() -> impl Strategy<Value = DateFieldAction> {
    prop_oneof![
        arb_editable_kind()
            .prop_map(|kind| DateFieldAction::Send(date_field::Event::FocusSegment(kind))),
        Just(DateFieldAction::Send(date_field::Event::BlurAll)),
        arb_editable_kind()
            .prop_map(|kind| DateFieldAction::Send(date_field::Event::IncrementSegment(kind))),
        arb_editable_kind()
            .prop_map(|kind| DateFieldAction::Send(date_field::Event::DecrementSegment(kind))),
        (arb_editable_kind(), arb_digit()).prop_map(|(kind, ch)| DateFieldAction::Send(
            date_field::Event::TypeIntoSegment(kind, ch)
        )),
        arb_editable_kind()
            .prop_map(|kind| DateFieldAction::Send(date_field::Event::TypeBufferCommit(kind))),
        Just(DateFieldAction::Send(date_field::Event::CompositionStart)),
        (arb_editable_kind(), "[0-9]{0,4}".prop_map(String::from)).prop_map(|(kind, text)| {
            DateFieldAction::Send(date_field::Event::CompositionEnd(kind, text))
        }),
        arb_editable_kind()
            .prop_map(|kind| DateFieldAction::Send(date_field::Event::ClearSegment(kind))),
        Just(DateFieldAction::Send(date_field::Event::ClearAll)),
        prop::option::of(arb_date())
            .prop_map(|value| DateFieldAction::Send(date_field::Event::SetValue(value))),
        prop::option::of(arb_date()).prop_map(DateFieldAction::SetControlledValue),
        any::<bool>().prop_map(DateFieldAction::SetDisabled),
        any::<bool>().prop_map(DateFieldAction::SetReadonly),
        any::<bool>().prop_map(DateFieldAction::SetInvalid),
    ]
}

fn arb_time_field_action() -> impl Strategy<Value = TimeFieldAction> {
    prop_oneof![
        arb_time_editable_kind()
            .prop_map(|kind| TimeFieldAction::Send(time_field::Event::FocusSegment { kind })),
        Just(TimeFieldAction::Send(time_field::Event::BlurAll)),
        arb_time_editable_kind()
            .prop_map(|kind| TimeFieldAction::Send(time_field::Event::IncrementSegment { kind })),
        arb_time_editable_kind()
            .prop_map(|kind| TimeFieldAction::Send(time_field::Event::DecrementSegment { kind })),
        (
            arb_time_editable_kind(),
            prop_oneof![arb_digit(), Just('a'), Just('p')]
        )
            .prop_map(|(kind, ch)| TimeFieldAction::Send(
                time_field::Event::TypeIntoSegment { kind, ch }
            )),
        arb_time_editable_kind().prop_map(|kind| {
            TimeFieldAction::Send(time_field::Event::TypeBufferCommit { kind })
        }),
        arb_time_editable_kind()
            .prop_map(|kind| TimeFieldAction::Send(time_field::Event::ClearSegment { kind })),
        Just(TimeFieldAction::Send(time_field::Event::ClearAll)),
        prop::option::of(arb_time())
            .prop_map(|value| TimeFieldAction::Send(time_field::Event::SetValue(value))),
        prop::option::of(arb_time()).prop_map(TimeFieldAction::SetControlledValue),
        any::<bool>().prop_map(TimeFieldAction::SetDisabled),
        any::<bool>().prop_map(TimeFieldAction::SetReadonly),
        any::<bool>().prop_map(TimeFieldAction::SetInvalid),
    ]
}

proptest! {
    #![proptest_config(super::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_date_field_event_sequences_preserve_invariants(
        actions in prop::collection::vec(arb_date_field_action(), 0..128),
    ) {
        let mut service = Service::<date_field::Machine>::new(
            date_field_props(),
            &Env::default(),
            &date_field::Messages::default(),
        );

        for action in actions {
            match action {
                DateFieldAction::Send(event) => {
                    drop(service.send(event));
                }

                DateFieldAction::SetControlledValue(value) => {
                    drop(service.set_props(date_field_props().value(value)));
                }

                DateFieldAction::SetDisabled(value) => {
                    drop(service.set_props(date_field_props().disabled(value)));
                }

                DateFieldAction::SetReadonly(value) => {
                    drop(service.set_props(date_field_props().readonly(value)));
                }

                DateFieldAction::SetInvalid(value) => {
                    drop(service.set_props(date_field_props().invalid(value)));
                }
            }

            let ctx = service.context();

            prop_assert_eq!(ctx.ids.id(), "date-field");

            for segment in &ctx.segments {
                prop_assert!(segment.min <= segment.max);

                if let Some(value) = segment.value {
                    prop_assert!(value >= segment.min);
                    prop_assert!(value <= segment.max);
                }
            }

            match service.state() {
                date_field::State::Idle => {}

                date_field::State::Focused(kind) => {
                    prop_assert!(kind.is_editable());
                    prop_assert_eq!(ctx.focused_segment, Some(*kind));
                }
            }

            if ctx.type_buffer.is_empty()
                && ctx.pending_controlled_value.is_none()
                && !ctx.value.is_controlled()
                && let Some(value) = ctx.value.get()
            {
                prop_assert_eq!(
                    ctx.segments
                        .iter()
                        .find(|segment| segment.kind == DateSegmentKind::Year)
                        .and_then(|segment| segment.value),
                    Some(value.year())
                );
                prop_assert_eq!(
                    ctx.segments
                        .iter()
                        .find(|segment| segment.kind == DateSegmentKind::Month)
                        .and_then(|segment| segment.value),
                    Some(i32::from(value.month()))
                );
                prop_assert_eq!(
                    ctx.segments
                        .iter()
                        .find(|segment| segment.kind == DateSegmentKind::Day)
                        .and_then(|segment| segment.value),
                    Some(i32::from(value.day()))
                );
            }
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_time_field_event_sequences_preserve_invariants(
        actions in prop::collection::vec(arb_time_field_action(), 0..128),
    ) {
        let mut service = Service::<time_field::Machine>::new(
            time_field_props(),
            &Env::default(),
            &time_field::Messages::default(),
        );

        for action in actions {
            match action {
                TimeFieldAction::Send(event) => {
                    drop(service.send(event));
                }

                TimeFieldAction::SetControlledValue(value) => {
                    drop(service.set_props(time_field_props().value(value)));
                }

                TimeFieldAction::SetDisabled(value) => {
                    drop(service.set_props(time_field_props().disabled(value)));
                }

                TimeFieldAction::SetReadonly(value) => {
                    drop(service.set_props(time_field_props().readonly(value)));
                }

                TimeFieldAction::SetInvalid(value) => {
                    drop(service.set_props(time_field_props().invalid(value)));
                }
            }

            let ctx = service.context();

            prop_assert_eq!(ctx.ids.id(), "time-field");

            for segment in &ctx.segments {
                prop_assert!(segment.min <= segment.max);

                if let Some(value) = segment.value {
                    prop_assert!(value >= segment.min);
                    prop_assert!(value <= segment.max);
                }
            }

            if matches!(ctx.hour_cycle, HourCycle::H23 | HourCycle::H24) {
                prop_assert!(
                    !ctx.segments
                        .iter()
                        .any(|segment| segment.kind == DateSegmentKind::DayPeriod)
                );
            }

            match service.state() {
                time_field::State::Idle => {}

                time_field::State::Focused(kind) => {
                    prop_assert!(kind.is_editable());
                    prop_assert_eq!(ctx.focused_segment, Some(*kind));
                }
            }

            if ctx.type_buffer.is_empty()
                && !ctx.value.is_controlled()
                && let Some(value) = ctx.value.get()
            {
                prop_assert_eq!(
                    ctx.segments
                        .iter()
                        .find(|segment| segment.kind == DateSegmentKind::Hour)
                        .and_then(|segment| segment.value),
                    Some(match ctx.hour_cycle {
                        HourCycle::H11 => i32::from(value.hour() % 12),
                        HourCycle::H12 => i32::from(if value.hour() % 12 == 0 { 12 } else { value.hour() % 12 }),
                        HourCycle::H23 => i32::from(value.hour()),
                        HourCycle::H24 => i32::from(if value.hour() == 0 { 24 } else { value.hour() }),
                    })
                );
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Calendar
// ────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum CalendarAction {
    Send(calendar::Event),
    SetDisabled(bool),
    SetReadonly(bool),
    SetMin(Option<CalendarDate>),
    SetMax(Option<CalendarDate>),
}

fn calendar_props() -> calendar::Props {
    calendar::Props::new().id("cal").today(date(2024, 1, 15))
}

fn calendar_props_multi() -> calendar::Props {
    calendar_props().selection_mode(SelectionMode::Multiple)
}

fn arb_calendar_date() -> impl Strategy<Value = CalendarDate> {
    (1900i32..=2100, 1u8..=12, 1u8..=28).prop_map(|(y, m, d)| date(y, m, d))
}

fn arb_calendar_key() -> impl Strategy<Value = KeyboardKey> {
    prop_oneof![
        Just(KeyboardKey::ArrowLeft),
        Just(KeyboardKey::ArrowRight),
        Just(KeyboardKey::ArrowUp),
        Just(KeyboardKey::ArrowDown),
        Just(KeyboardKey::Home),
        Just(KeyboardKey::End),
        Just(KeyboardKey::PageUp),
        Just(KeyboardKey::PageDown),
        Just(KeyboardKey::Enter),
        Just(KeyboardKey::Space),
    ]
}

fn arb_calendar_event() -> impl Strategy<Value = calendar::Event> {
    prop_oneof![
        arb_calendar_date().prop_map(|date| calendar::Event::FocusDate { date }),
        arb_calendar_date().prop_map(|date| calendar::Event::SelectDate { date }),
        arb_calendar_date().prop_map(|date| calendar::Event::ToggleDate { date }),
        Just(calendar::Event::NextMonth),
        Just(calendar::Event::PrevMonth),
        Just(calendar::Event::NextYear),
        Just(calendar::Event::PrevYear),
        (1u8..=12).prop_map(|month| calendar::Event::SetMonth { month }),
        (1900i32..=2100).prop_map(|year| calendar::Event::SetYear { year }),
        Just(calendar::Event::FocusIn),
        Just(calendar::Event::FocusOut),
        (arb_calendar_key(), any::<bool>())
            .prop_map(|(key, shift)| calendar::Event::KeyDown { key, shift }),
    ]
}

fn arb_calendar_action() -> impl Strategy<Value = CalendarAction> {
    prop_oneof![
        arb_calendar_event().prop_map(CalendarAction::Send),
        any::<bool>().prop_map(CalendarAction::SetDisabled),
        any::<bool>().prop_map(CalendarAction::SetReadonly),
        prop::option::of(arb_calendar_date()).prop_map(CalendarAction::SetMin),
        prop::option::of(arb_calendar_date()).prop_map(CalendarAction::SetMax),
    ]
}

fn apply_calendar_action(
    service: &mut Service<calendar::Machine>,
    action: CalendarAction,
    base_props: &calendar::Props,
) {
    match action {
        CalendarAction::Send(event) => {
            drop(service.send(event));
        }

        CalendarAction::SetDisabled(value) => {
            drop(service.set_props(base_props.clone().disabled(value)));
        }

        CalendarAction::SetReadonly(value) => {
            drop(service.set_props(base_props.clone().readonly(value)));
        }

        CalendarAction::SetMin(value) => {
            drop(service.set_props(base_props.clone().min(value)));
        }

        CalendarAction::SetMax(value) => {
            drop(service.set_props(base_props.clone().max(value)));
        }
    }
}

fn assert_calendar_invariants(service: &Service<calendar::Machine>) {
    let ctx = service.context();

    assert!(ctx.visible_month >= 1, "visible_month must be 1-based");
    assert!(ctx.visible_month <= 12, "visible_month must be 1..=12");
    assert!(ctx.visible_months >= 1, "visible_months must be >= 1");
    assert!(
        matches!(
            ctx.first_day_of_week,
            Weekday::Sunday
                | Weekday::Monday
                | Weekday::Tuesday
                | Weekday::Wednesday
                | Weekday::Thursday
                | Weekday::Friday
                | Weekday::Saturday,
        ),
        "first_day_of_week must be a valid weekday",
    );

    if let Some(min) = &ctx.min {
        assert!(
            !matches!(ctx.focused_date.compare(min), core::cmp::Ordering::Less),
            "focused_date must be >= min",
        );
    }

    if let Some(max) = &ctx.max {
        assert!(
            !matches!(ctx.focused_date.compare(max), core::cmp::Ordering::Greater),
            "focused_date must be <= max",
        );
    }

    if let Some(cap) = ctx.max_selected {
        assert!(
            ctx.selected_dates.get().len() <= cap,
            "selected_dates exceeded max_selected cap",
        );
    }

    let weeks = ctx.weeks();

    if !weeks.is_empty() {
        assert_eq!(weeks.len(), 6, "grid must always render 6 weeks");
        for row in &weeks {
            assert_eq!(row.len(), 7, "every row must contain 7 days");
        }
    }

    assert!(matches!(
        service.state(),
        calendar::State::Idle | calendar::State::Focused,
    ));
}

proptest! {
    #![proptest_config(super::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_calendar_single_select_preserves_invariants(
        actions in prop::collection::vec(arb_calendar_action(), 0..64),
    ) {
        let base = calendar_props();

        let mut service = Service::<calendar::Machine>::new(
            base.clone(),
            &Env::default(),
            &calendar::Messages::default(),
        );

        for action in actions {
            apply_calendar_action(&mut service, action, &base);

            assert_calendar_invariants(&service);
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_calendar_multi_select_preserves_invariants(
        actions in prop::collection::vec(arb_calendar_action(), 0..64),
        max_selected in prop::option::of(1usize..=8),
    ) {
        let base = calendar_props_multi().max_selected(max_selected);

        let mut service = Service::<calendar::Machine>::new(
            base.clone(),
            &Env::default(),
            &calendar::Messages::default(),
        );

        for action in actions {
            apply_calendar_action(&mut service, action, &base);

            assert_calendar_invariants(&service);
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_calendar_page_behavior_advances_one_or_visible_months(
        visible in 1usize..=4,
        behavior in prop_oneof![Just(PageBehavior::Visible), Just(PageBehavior::Single)],
        steps in 0u8..=20,
    ) {
        let base = calendar_props().visible_months(visible).page_behavior(behavior);

        let mut service = Service::<calendar::Machine>::new(
            base.clone(),
            &Env::default(),
            &calendar::Messages::default(),
        );

        let start_month = i32::from(service.context().visible_month);
        let start_year = service.context().visible_year;

        for _ in 0..steps {
            drop(service.send(calendar::Event::NextMonth));
        }

        let step = match behavior {
            PageBehavior::Visible => i32::try_from(visible).unwrap_or(1),
            PageBehavior::Single => 1,
        };

        let expected_total = start_month - 1 + step * i32::from(steps);
        let expected_month = expected_total.rem_euclid(12) + 1;
        let expected_year = start_year + expected_total.div_euclid(12);

        prop_assert_eq!(service.context().visible_month, expected_month as u8);
        prop_assert_eq!(service.context().visible_year, expected_year);
    }
}
