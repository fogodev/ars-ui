use ars_components::input::pin_input;
use ars_core::{Direction, Env, HtmlAttr, Service};
use proptest::prelude::*;

fn arb_pin_mode() -> impl Strategy<Value = pin_input::Mode> {
    prop_oneof![
        Just(pin_input::Mode::Numeric),
        Just(pin_input::Mode::Alphanumeric),
        Just(pin_input::Mode::Password),
    ]
}

fn arb_pin_input_props() -> impl Strategy<Value = pin_input::Props> {
    (
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        arb_pin_mode(),
    )
        .prop_map(
            |(disabled, invalid, otp, mask, required, auto_submit, mode)| pin_input::Props {
                id: "pin-input".to_string(),
                value: None,
                default_value: Vec::new(),
                length: 4,
                disabled,
                invalid,
                otp,
                mask,
                placeholder: Some("_".to_string()),
                mode,
                name: Some("pin".to_string()),
                form: Some("form".to_string()),
                required,
                readonly: false,
                select_on_focus: false,
                blur_on_complete: false,
                auto_submit,
                on_value_complete: None,
                dir: Direction::Ltr,
            },
        )
}

fn arb_pin_input_event() -> impl Strategy<Value = pin_input::Event> {
    prop_oneof![
        (0_usize..4, any::<bool>())
            .prop_map(|(index, is_keyboard)| pin_input::Event::Focus { index, is_keyboard }),
        Just(pin_input::Event::Blur),
        (
            0_usize..4,
            "[0-9a-z]".prop_map(|s| s.chars().next().unwrap_or('0'))
        )
            .prop_map(|(index, c)| pin_input::Event::InputChar { index, char: c }),
        (0_usize..4).prop_map(|index| pin_input::Event::DeleteChar { index }),
        "[0-9a-z]{0,8}".prop_map(pin_input::Event::Paste),
        Just(pin_input::Event::Clear),
        Just(pin_input::Event::FocusNext),
        Just(pin_input::Event::FocusPrev),
        Just(pin_input::Event::CompositionStart),
        Just(pin_input::Event::CompositionEnd),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_pin_input_event_sequences_preserve_invariants(
        props in arb_pin_input_props(),
        events in prop::collection::vec(arb_pin_input_event(), 0..128),
    ) {
        let mut service = Service::<pin_input::Machine>::new(
            props,
            &Env::default(),
            &pin_input::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            let ctx = service.context();

            // Cell vector always has the configured length
            prop_assert_eq!(ctx.value.get().len(), ctx.length);

            // Complete iff every cell is non-empty (for length > 0).
            let all_filled = ctx.length > 0 && ctx.value.get().iter().all(|cell| !cell.is_empty());

            prop_assert_eq!(ctx.complete, all_filled);

            // Hidden input value equals the joined cell strings.
            let hidden_attrs = service.connect(&|_| {}).hidden_input_attrs();

            let joined = ctx.value.get().join("");

            prop_assert_eq!(
                hidden_attrs.get(&HtmlAttr::Value),
                Some(joined.as_str())
            );
        }
    }
}
