use ars_components::date_time::{date_field::DateSegmentKind, time_field};
use ars_core::{Env, Service};
use ars_i18n::{HourCycle, Time};
use proptest::prelude::*;

use super::helpers::{arb_digit, arb_time};

#[derive(Clone, Debug)]
enum TimeFieldAction {
    Send(time_field::Event),
    SetControlledValue(Option<Time>),
    SetDisabled(bool),
    SetReadonly(bool),
    SetInvalid(bool),
}

fn time_field_props() -> time_field::Props {
    time_field::Props::new()
        .id("time-field")
        .label("Time")
        .hour_cycle(Some(HourCycle::H12))
        .granularity(time_field::TimeGranularity::Second)
}

fn arb_time_editable_kind() -> impl Strategy<Value = DateSegmentKind> {
    prop_oneof![
        Just(DateSegmentKind::Hour),
        Just(DateSegmentKind::Minute),
        Just(DateSegmentKind::Second),
        Just(DateSegmentKind::DayPeriod),
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
    #![proptest_config(crate::common::proptest_config())]

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
