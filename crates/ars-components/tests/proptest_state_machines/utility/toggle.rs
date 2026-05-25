use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_toggle_event_sequences_preserve_invariants(
        props in arb_toggle_props(),
        events in prop::collection::vec(arb_toggle_event(), 0..128),
    ) {
        let mut service = Service::<utility_core::toggle::Machine>::new(
            props,
            &Env::default(),
            &utility_core::toggle::Messages,
        );

        for event in events {
            let before_pressed = *service.context().pressed.get();

            let before_disabled = service.context().disabled;

            let value_event = matches!(
                event,
                utility_core::toggle::Event::Toggle | utility_core::toggle::Event::TurnOn | utility_core::toggle::Event::TurnOff
            );

            drop(service.send(event));

            let state = service.state();
            let ctx = service.context();

            prop_assert_eq!(matches!(state, utility_core::toggle::State::On), *ctx.pressed.get());
            prop_assert!(
                !ctx.focus_visible || ctx.focused,
                "focus-visible cannot outlive focus"
            );

            if before_disabled && value_event {
                prop_assert_eq!(
                    *ctx.pressed.get(),
                    before_pressed,
                    "disabled toggle must not change pressed value"
                );
            }
        }
    }
}
