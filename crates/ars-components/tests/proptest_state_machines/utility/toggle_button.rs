use super::*;

fn arb_toggle_button_props() -> impl Strategy<Value = utility_core::toggle_button::Props> {
    (
        prop::option::of(any::<bool>()),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(pressed, default_pressed, disabled)| utility_core::toggle_button::Props {
                id: "toggle-button".to_string(),
                pressed,
                default_pressed,
                disabled,
                invalid: false,
                required: false,
                value: Some("on".to_string()),
                name: Some("power".to_string()),
                form: None,
                prevent_focus_on_press: false,
                on_change: None,
                on_hover_start: None,
                on_hover_end: None,
                on_hover_change: None,
            },
        )
}

fn arb_toggle_button_event() -> impl Strategy<Value = utility_core::toggle_button::Event> {
    prop_oneof![
        Just(utility_core::toggle_button::Event::Toggle),
        Just(utility_core::toggle_button::Event::Press),
        Just(utility_core::toggle_button::Event::Release),
        any::<bool>()
            .prop_map(|is_keyboard| utility_core::toggle_button::Event::Focus { is_keyboard }),
        Just(utility_core::toggle_button::Event::Blur),
        any::<bool>().prop_map(utility_core::toggle_button::Event::SetPressed),
        any::<bool>().prop_map(utility_core::toggle_button::Event::SetDisabled),
        Just(utility_core::toggle_button::Event::Reset),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_toggle_button_event_sequences_preserve_invariants(
        props in arb_toggle_button_props(),
        events in prop::collection::vec(arb_toggle_button_event(), 0..128),
    ) {
        let mut service = Service::<utility_core::toggle_button::Machine>::new(
            props,
            &Env::default(),
            &utility_core::toggle_button::Messages,
        );

        for event in events {
            drop(service.send(event));

            let ctx = service.context();

            prop_assert!(!ctx.focus_visible || ctx.focused);
        }
    }
}
