use ars_components::date_time::date_picker;
use ars_core::{ComponentPart, ConnectApi, Env, KeyboardKey, Service};
use ars_i18n::CalendarDate;
use proptest::prelude::*;

use super::helpers::arb_date;

#[derive(Clone, Debug)]
enum DatePickerAction {
    Send(date_picker::Event),
    SetControlledValue(Option<CalendarDate>),
    SetControlledOpen(Option<bool>),
    SetDisabled(bool),
}

fn date_picker_props() -> date_picker::Props {
    date_picker::Props {
        id: "date-picker".to_string(),
        label: "Date".to_string(),
        ..date_picker::Props::default()
    }
}

fn arb_date_picker_event() -> impl Strategy<Value = date_picker::Event> {
    prop_oneof![
        Just(date_picker::Event::Open),
        Just(date_picker::Event::Close),
        Just(date_picker::Event::Toggle),
        arb_date().prop_map(|date| date_picker::Event::SelectDate { date }),
        "[0-9/]{0,12}".prop_map(|value| date_picker::Event::InputChange { value }),
        Just(date_picker::Event::FocusIn),
        Just(date_picker::Event::FocusOut),
        Just(date_picker::Event::KeyDown {
            key: KeyboardKey::Escape,
        }),
        Just(date_picker::Event::KeyDown {
            key: KeyboardKey::ArrowDown,
        }),
    ]
}

fn arb_date_picker_action() -> impl Strategy<Value = DatePickerAction> {
    prop_oneof![
        arb_date_picker_event().prop_map(DatePickerAction::Send),
        prop_oneof![Just(None), arb_date().prop_map(Some)]
            .prop_map(DatePickerAction::SetControlledValue),
        prop_oneof![Just(None), Just(Some(true)), Just(Some(false))]
            .prop_map(DatePickerAction::SetControlledOpen),
        any::<bool>().prop_map(DatePickerAction::SetDisabled),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    /// Random event and controlled-prop sequences keep the picker's invariants:
    /// the component ID stays stable, the state is always one of its two
    /// variants, and connecting every anatomy part never panics (exercising
    /// `parse_date` on arbitrary text in particular).
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_date_picker_sequences_preserve_invariants(
        actions in prop::collection::vec(arb_date_picker_action(), 0..128),
    ) {
        let mut service = Service::<date_picker::Machine>::new(
            date_picker_props(),
            &Env::default(),
            &date_picker::Messages::default(),
        );

        for action in actions {
            match action {
                DatePickerAction::Send(event) => {
                    drop(service.send(event));
                }

                DatePickerAction::SetControlledValue(value) => {
                    drop(service.set_props(date_picker::Props {
                        value: Some(value),
                        ..date_picker_props()
                    }));
                }

                DatePickerAction::SetControlledOpen(open) => {
                    drop(service.set_props(date_picker::Props {
                        open,
                        ..date_picker_props()
                    }));
                }

                DatePickerAction::SetDisabled(disabled) => {
                    drop(service.set_props(date_picker::Props {
                        disabled,
                        ..date_picker_props()
                    }));
                }
            }

            prop_assert_eq!(service.context().ids.id(), "date-picker");
            prop_assert!(matches!(
                service.state(),
                date_picker::State::Open | date_picker::State::Closed,
            ));

            let send = |_event: date_picker::Event| {};

            let api = service.connect(&send);

            for part in date_picker::Part::all() {
                drop(api.part_attrs(part));
            }
        }
    }
}
