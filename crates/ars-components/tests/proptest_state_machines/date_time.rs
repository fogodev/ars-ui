use ars_components::date_time::date_field::{self, DateSegmentKind};
use ars_core::{Env, Service};
use ars_i18n::CalendarDate;
use proptest::prelude::*;

#[derive(Clone, Debug)]
enum DateFieldAction {
    Send(date_field::Event),
    SetControlledValue(Option<CalendarDate>),
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

fn arb_date() -> impl Strategy<Value = CalendarDate> {
    (1900i32..=2100, 1u8..=12, 1u8..=28).prop_map(|(year, month, day)| date(year, month, day))
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
}
