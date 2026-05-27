use ars_components::input::switch;
use ars_core::{AriaAttr, Direction, Env, HtmlAttr, Service};
use proptest::prelude::*;

fn arb_switch_props() -> impl Strategy<Value = switch::Props> {
    (
        prop::option::of(any::<bool>()),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        prop::option::of("[a-z]{1,8}".prop_map(String::from)),
        prop::option::of("[a-z]{1,8}".prop_map(String::from)),
        "[a-z]{1,8}".prop_map(String::from),
        prop::option::of("[a-z]{1,8}".prop_map(String::from)),
        any::<bool>(),
    )
        .prop_map(
            |(
                checked,
                default_checked,
                disabled,
                required,
                invalid,
                readonly,
                name,
                form,
                value,
                label,
                rtl,
            )| switch::Props {
                id: "switch".to_string(),
                checked,
                default_checked,
                disabled,
                required,
                invalid,
                readonly,
                name,
                form,
                value,
                label,
                dir: if rtl { Direction::Rtl } else { Direction::Ltr },
                on_checked_change: None,
            },
        )
}

fn arb_switch_event() -> impl Strategy<Value = switch::Event> {
    prop_oneof![
        Just(switch::Event::Toggle),
        Just(switch::Event::TurnOn),
        Just(switch::Event::TurnOff),
        Just(switch::Event::Reset),
        prop::option::of(any::<bool>()).prop_map(switch::Event::SetValue),
        Just(switch::Event::SetProps),
        any::<bool>().prop_map(switch::Event::SetHasDescription),
        any::<bool>().prop_map(|is_keyboard| switch::Event::Focus { is_keyboard }),
        Just(switch::Event::Blur),
    ]
}

const fn switch_state_checked(state: switch::State) -> bool {
    match state {
        switch::State::Off => false,
        switch::State::On => true,
    }
}

const fn switch_aria_checked_token(state: switch::State) -> &'static str {
    match state {
        switch::State::Off => "false",
        switch::State::On => "true",
    }
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_switch_event_sequences_preserve_invariants(
        props in arb_switch_props(),
        events in prop::collection::vec(arb_switch_event(), 0..128),
    ) {
        let mut service = Service::<switch::Machine>::new(
            props,
            &Env::default(),
            &switch::Messages,
        );

        for event in events {
            drop(service.send(event));

            let state = *service.state();

            prop_assert_eq!(service.context().ids.id(), "switch");
            prop_assert_eq!(service.context().checked.get(), &switch_state_checked(state));

            let control_attrs = service.connect(&|_| {}).control_attrs();

            prop_assert_eq!(control_attrs.get(&HtmlAttr::Role), Some("switch"));
            prop_assert_eq!(
                control_attrs.get(&HtmlAttr::Aria(AriaAttr::Checked)),
                Some(switch_aria_checked_token(state))
            );

            let hidden_input_attrs = service.connect(&|_| {}).hidden_input_attrs();

            if service.context().disabled {
                prop_assert!(hidden_input_attrs.contains(&HtmlAttr::Disabled));
            }

            if service.context().required {
                prop_assert!(hidden_input_attrs.contains(&HtmlAttr::Required));
            }

            let described_by = match (service.context().has_description, service.context().invalid) {
                (false, false) => None,
                (true, false) => Some("switch-description"),
                (false, true) => Some("switch-error-message"),
                (true, true) => Some("switch-description switch-error-message"),
            };

            prop_assert_eq!(
                control_attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
                described_by,
            );
        }
    }
}
