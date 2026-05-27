use ars_components::input::password_input;
use ars_core::{Env, HtmlAttr, Service};
use proptest::prelude::*;

use super::arb_short_text;

fn arb_password_input_props() -> impl Strategy<Value = password_input::Props> {
    (
        prop::option::of(arb_short_text()),
        arb_short_text(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(value, default_value, disabled, required, invalid, readonly, default_visible)| {
                password_input::Props {
                    id: "password-input".to_string(),
                    value,
                    default_value,
                    disabled,
                    required,
                    invalid,
                    readonly,
                    default_visible,
                    placeholder: Some("Password".to_string()),
                    name: Some("password".to_string()),
                    form: Some("form".to_string()),
                    autocomplete: Some("current-password".to_string()),
                }
            },
        )
}

fn arb_password_input_event() -> impl Strategy<Value = password_input::Event> {
    prop_oneof![
        Just(password_input::Event::ToggleVisibility),
        any::<bool>().prop_map(password_input::Event::SetVisibility),
        any::<bool>().prop_map(|is_keyboard| password_input::Event::Focus { is_keyboard }),
        Just(password_input::Event::Blur),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_password_input_event_sequences_preserve_invariants(
        props in arb_password_input_props(),
        events in prop::collection::vec(arb_password_input_event(), 0..128),
    ) {
        let mut service = Service::<password_input::Machine>::new(
            props,
            &Env::default(),
            &password_input::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            let ctx = service.context();
            let state = service.state();

            // visibility flag tracks state
            match state {
                password_input::State::Masked => prop_assert!(!ctx.visible),
                password_input::State::Visible => prop_assert!(ctx.visible),
            }

            let input_attrs = service.connect(&|_| {}).input_attrs();

            let ty = if ctx.visible { "text" } else { "password" };

            prop_assert_eq!(input_attrs.get(&HtmlAttr::Type), Some(ty));

            if ctx.disabled {
                prop_assert!(input_attrs.contains(&HtmlAttr::Disabled));
            }
        }
    }
}
