use ars_components::date_time::{date_field::DateSegmentKind, date_time_picker};
use ars_core::{ComponentPart, ConnectApi, Env, KeyboardKey, Service};
use ars_i18n::{CalendarDateTime, HourCycle};
use proptest::prelude::*;

use super::helpers::{arb_date, arb_time};

fn arb_datetime() -> impl Strategy<Value = CalendarDateTime> {
    (arb_date(), arb_time()).prop_map(|(date, time)| CalendarDateTime::new(date, time))
}

#[derive(Clone, Debug)]
enum DateTimePickerAction {
    Send(date_time_picker::Event),
    SetControlledValue(Option<CalendarDateTime>),
    SetDisabled(bool),
    SetReadonly(bool),
}

fn date_time_picker_props() -> date_time_picker::Props {
    date_time_picker::Props {
        id: "date-time-picker".to_string(),
        label: "Appointment".to_string(),
        ..date_time_picker::Props::default()
    }
}

fn arb_date_time_picker_segment() -> impl Strategy<Value = DateSegmentKind> {
    prop_oneof![
        Just(DateSegmentKind::Year),
        Just(DateSegmentKind::Month),
        Just(DateSegmentKind::Day),
        Just(DateSegmentKind::Hour),
        Just(DateSegmentKind::Minute),
        Just(DateSegmentKind::DayPeriod),
    ]
}

fn arb_date_time_picker_event() -> impl Strategy<Value = date_time_picker::Event> {
    prop_oneof![
        Just(date_time_picker::Event::Open),
        Just(date_time_picker::Event::Close),
        Just(date_time_picker::Event::Toggle),
        Just(date_time_picker::Event::FocusIn),
        Just(date_time_picker::Event::FocusOut),
        Just(date_time_picker::Event::FocusNextSegment),
        Just(date_time_picker::Event::FocusPrevSegment),
        Just(date_time_picker::Event::ClearAll),
        arb_date().prop_map(date_time_picker::Event::CalendarSelectDate),
        arb_date_time_picker_segment().prop_map(date_time_picker::Event::FocusSegment),
        arb_date_time_picker_segment()
            .prop_map(|segment| date_time_picker::Event::IncrementSegment { segment }),
        arb_date_time_picker_segment()
            .prop_map(|segment| date_time_picker::Event::DecrementSegment { segment }),
        (arb_date_time_picker_segment(), 0i32..=60)
            .prop_map(|(segment, value)| date_time_picker::Event::SegmentChange { segment, value }),
        (arb_date_time_picker_segment(), prop::char::range('0', '9'))
            .prop_map(|(segment, ch)| date_time_picker::Event::TypeIntoSegment { segment, ch }),
        arb_date_time_picker_segment()
            .prop_map(|segment| date_time_picker::Event::ClearSegment { segment }),
        prop_oneof![Just(None), arb_datetime().prop_map(Some)]
            .prop_map(date_time_picker::Event::ValueCommit),
        Just(date_time_picker::Event::KeyDown {
            key: KeyboardKey::Escape,
        }),
        Just(date_time_picker::Event::KeyDown {
            key: KeyboardKey::ArrowDown,
        }),
    ]
}

fn arb_date_time_picker_action() -> impl Strategy<Value = DateTimePickerAction> {
    prop_oneof![
        arb_date_time_picker_event().prop_map(DateTimePickerAction::Send),
        prop_oneof![Just(None), arb_datetime().prop_map(Some)]
            .prop_map(DateTimePickerAction::SetControlledValue),
        any::<bool>().prop_map(DateTimePickerAction::SetDisabled),
        any::<bool>().prop_map(DateTimePickerAction::SetReadonly),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    /// Random event and controlled-prop sequences keep the picker's invariants:
    /// the component ID stays stable, the state is always one of its three
    /// variants, every segment's value stays within its `[min, max]` range, a
    /// 24-hour cycle never grows a day-period segment, the focused segment is
    /// always editable, a committed value never falls outside `[min, max]`, and
    /// connecting every anatomy part never panics.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_date_time_picker_sequences_preserve_invariants(
        actions in prop::collection::vec(arb_date_time_picker_action(), 0..128),
    ) {
        let mut service = Service::<date_time_picker::Machine>::new(
            date_time_picker_props(),
            &Env::default(),
            &date_time_picker::Messages::default(),
        );

        for action in actions {
            match action {
                DateTimePickerAction::Send(event) => {
                    drop(service.send(event));
                }

                DateTimePickerAction::SetControlledValue(value) => {
                    drop(service.set_props(date_time_picker::Props {
                        value: Some(value),
                        ..date_time_picker_props()
                    }));
                }

                DateTimePickerAction::SetDisabled(disabled) => {
                    drop(service.set_props(date_time_picker::Props {
                        disabled,
                        ..date_time_picker_props()
                    }));
                }

                DateTimePickerAction::SetReadonly(readonly) => {
                    drop(service.set_props(date_time_picker::Props {
                        readonly,
                        ..date_time_picker_props()
                    }));
                }
            }

            let ctx = service.context();

            prop_assert_eq!(ctx.ids.id(), "date-time-picker");
            prop_assert!(matches!(
                service.state(),
                date_time_picker::State::Idle
                    | date_time_picker::State::Focused
                    | date_time_picker::State::Open,
            ));

            for segment in ctx.all_segments() {
                prop_assert!(segment.min <= segment.max);

                if let Some(value) = segment.value {
                    prop_assert!(value >= segment.min);
                    prop_assert!(value <= segment.max);
                }
            }

            if matches!(ctx.hour_cycle, HourCycle::H23 | HourCycle::H24) {
                prop_assert!(
                    !ctx.all_segments()
                        .any(|segment| segment.kind == DateSegmentKind::DayPeriod)
                );
            }

            if let Some(kind) = ctx.focused_segment {
                prop_assert!(
                    ctx.segment(kind).is_some_and(|segment| segment.is_editable)
                );
            }

            let send = |_event: date_time_picker::Event| {};

            let api = service.connect(&send);

            for part in date_time_picker::Part::all() {
                drop(api.part_attrs(part));
            }
        }
    }
}
