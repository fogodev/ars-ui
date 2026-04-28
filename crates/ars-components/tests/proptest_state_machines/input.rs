use ars_components::input::checkbox;
use ars_core::{AriaAttr, Env, HtmlAttr, Service};
use proptest::prelude::*;

fn arb_checkbox_state() -> impl Strategy<Value = checkbox::State> {
    prop_oneof![
        Just(checkbox::State::Unchecked),
        Just(checkbox::State::Checked),
        Just(checkbox::State::Indeterminate),
    ]
}

fn arb_checkbox_props() -> impl Strategy<Value = checkbox::Props> {
    (
        prop::option::of(arb_checkbox_state()),
        arb_checkbox_state(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        prop::option::of("[a-z]{1,8}".prop_map(String::from)),
        prop::option::of("[a-z]{1,8}".prop_map(String::from)),
        "[a-z]{1,8}".prop_map(String::from),
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
            )| {
                checkbox::Props {
                    id: "checkbox".to_string(),
                    checked,
                    default_checked,
                    disabled,
                    required,
                    invalid,
                    readonly,
                    name,
                    form,
                    value,
                    on_checked_change: None,
                }
            },
        )
}

fn arb_checkbox_event() -> impl Strategy<Value = checkbox::Event> {
    prop_oneof![
        Just(checkbox::Event::Toggle),
        Just(checkbox::Event::Check),
        Just(checkbox::Event::Uncheck),
        prop::option::of(arb_checkbox_state()).prop_map(checkbox::Event::SetValue),
        Just(checkbox::Event::SetProps),
        any::<bool>().prop_map(checkbox::Event::SetHasDescription),
        any::<bool>().prop_map(|is_keyboard| checkbox::Event::Focus { is_keyboard }),
        Just(checkbox::Event::Blur),
    ]
}

const fn aria_checked_token(state: checkbox::State) -> &'static str {
    match state {
        checkbox::State::Unchecked => "false",
        checkbox::State::Checked => "true",
        checkbox::State::Indeterminate => "mixed",
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000)
    ))]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_checkbox_event_sequences_preserve_invariants(
        props in arb_checkbox_props(),
        events in prop::collection::vec(arb_checkbox_event(), 0..128),
    ) {
        let mut service = Service::<checkbox::Machine>::new(
            props,
            &Env::default(),
            &checkbox::Messages,
        );

        for event in events {
            drop(service.send(event));

            let state = *service.state();

            prop_assert_eq!(service.context().ids.id(), "checkbox");
            prop_assert_eq!(service.context().checked.get(), &state);

            let control_attrs = service.connect(&|_| {}).control_attrs();

            prop_assert_eq!(
                control_attrs.get(&HtmlAttr::Aria(AriaAttr::Checked)),
                Some(aria_checked_token(state))
            );

            let hidden_input_attrs = service.connect(&|_| {}).hidden_input_attrs();

            if service.context().disabled {
                prop_assert!(hidden_input_attrs.contains(&HtmlAttr::Disabled));
            }

            if service.context().required {
                prop_assert!(hidden_input_attrs.contains(&HtmlAttr::Required));
            }
        }
    }
}
