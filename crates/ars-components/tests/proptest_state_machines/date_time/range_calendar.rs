use ars_components::date_time::range_calendar;
use ars_core::{Env, KeyboardKey, Service};
use ars_i18n::{CalendarDate, DateRange};
use proptest::prelude::*;

use super::helpers::{arb_calendar_date, date};

#[derive(Clone, Debug)]
enum RangeCalendarAction {
    Send(range_calendar::Event),
    SetDisabled(bool),
    SetReadonly(bool),
    SetMin(Option<CalendarDate>),
    SetMax(Option<CalendarDate>),
}

fn range_calendar_props() -> range_calendar::Props {
    range_calendar::Props::new()
        .id("range-calendar")
        .today(date(2024, 1, 15))
        .max_range_days(Some(14))
}

fn arb_range_calendar_key() -> impl Strategy<Value = KeyboardKey> {
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

fn arb_range_calendar_event() -> impl Strategy<Value = range_calendar::Event> {
    prop_oneof![
        arb_calendar_date().prop_map(|date| range_calendar::Event::FocusDate { date }),
        arb_calendar_date().prop_map(|date| range_calendar::Event::SelectDate { date }),
        arb_calendar_date().prop_map(|date| range_calendar::Event::HoverDate { date }),
        Just(range_calendar::Event::HoverEnd),
        Just(range_calendar::Event::NextMonth),
        Just(range_calendar::Event::PrevMonth),
        Just(range_calendar::Event::NextYear),
        Just(range_calendar::Event::PrevYear),
        (1u8..=12).prop_map(|month| range_calendar::Event::SetMonth { month }),
        (1900i32..=2100).prop_map(|year| range_calendar::Event::SetYear { year }),
        Just(range_calendar::Event::FocusIn),
        Just(range_calendar::Event::FocusOut),
        (arb_range_calendar_key(), any::<bool>())
            .prop_map(|(key, shift)| range_calendar::Event::KeyDown { key, shift }),
    ]
}

fn arb_range_calendar_action() -> impl Strategy<Value = RangeCalendarAction> {
    prop_oneof![
        arb_range_calendar_event().prop_map(RangeCalendarAction::Send),
        any::<bool>().prop_map(RangeCalendarAction::SetDisabled),
        any::<bool>().prop_map(RangeCalendarAction::SetReadonly),
        prop::option::of(arb_calendar_date()).prop_map(RangeCalendarAction::SetMin),
        prop::option::of(arb_calendar_date()).prop_map(RangeCalendarAction::SetMax),
    ]
}

fn apply_range_calendar_action(
    service: &mut Service<range_calendar::Machine>,
    action: RangeCalendarAction,
    base_props: &range_calendar::Props,
) {
    match action {
        RangeCalendarAction::Send(event) => {
            drop(service.send(event));
        }

        RangeCalendarAction::SetDisabled(value) => {
            drop(service.set_props(base_props.clone().disabled(value)));
        }

        RangeCalendarAction::SetReadonly(value) => {
            drop(service.set_props(base_props.clone().readonly(value)));
        }

        RangeCalendarAction::SetMin(value) => {
            drop(service.set_props(base_props.clone().min(value)));
        }

        RangeCalendarAction::SetMax(value) => {
            drop(service.set_props(base_props.clone().max(value)));
        }
    }
}

fn assert_range_calendar_invariants(service: &Service<range_calendar::Machine>) {
    let ctx = service.context();

    assert!(ctx.visible_month >= 1, "visible_month must be 1-based");
    assert!(ctx.visible_month <= 12, "visible_month must be 1..=12");
    assert!(ctx.visible_months >= 1, "visible_months must be >= 1");

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

    if let Some(range) = ctx.value.get() {
        assert!(
            !matches!(
                range.start.compare(&range.end),
                core::cmp::Ordering::Greater,
            ),
            "range must be normalized",
        );
        assert!(
            ctx.range_is_allowed(range),
            "completed range must satisfy span constraints",
        );
    }

    if ctx.anchor_date.is_none() {
        assert!(
            ctx.hovering_date.is_none(),
            "hovering_date must not outlive a pending anchor",
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
        range_calendar::State::Idle | range_calendar::State::Focused,
    ));
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_range_calendar_sequences_preserve_invariants(
        actions in prop::collection::vec(arb_range_calendar_action(), 0..64),
        allow_single in any::<bool>(),
        min_range_days in prop::option::of(1u32..=5),
        max_range_days in prop::option::of(6u32..=21),
    ) {
        let base = range_calendar_props()
            .allow_single_date_range(allow_single)
            .min_range_days(min_range_days)
            .max_range_days(max_range_days);

        let mut service = Service::<range_calendar::Machine>::new(
            base.clone(),
            &Env::default(),
            &range_calendar::Messages::default(),
        );

        for action in actions {
            apply_range_calendar_action(&mut service, action, &base);

            assert_range_calendar_invariants(&service);
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_range_calendar_accepts_only_allowed_completed_ranges(
        first in arb_calendar_date(),
        second in arb_calendar_date(),
        max_days in 1u32..=20,
    ) {
        let base = range_calendar_props().max_range_days(Some(max_days));

        let mut service = Service::<range_calendar::Machine>::new(
            base,
            &Env::default(),
            &range_calendar::Messages::default(),
        );

        drop(service.send(range_calendar::Event::SelectDate { date: first.clone() }));
        drop(service.send(range_calendar::Event::SelectDate { date: second.clone() }));

        if let Some(expected) = DateRange::normalized(first, second)
            && expected
                .start
                .days_until(&expected.end)
                .ok()
                .and_then(|days| u32::try_from(days).ok())
                .and_then(|days| days.checked_add(1))
                .is_some_and(|days| days <= max_days)
        {
            prop_assert_eq!(service.context().value.get(), &Some(expected));
        } else {
            prop_assert!(service.context().value.get().is_none());
            prop_assert!(service.context().anchor_date.is_some());
        }
    }
}
