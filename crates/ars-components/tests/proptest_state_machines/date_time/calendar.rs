use ars_components::date_time::calendar::{self, PageBehavior, SelectionMode};
use ars_core::{Env, KeyboardKey, Service};
use ars_i18n::{CalendarDate, Weekday};
use proptest::prelude::*;

use super::helpers::date;

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
    #![proptest_config(crate::common::proptest_config())]

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
