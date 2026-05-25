use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_swap_event_sequences_preserve_invariants(
        props in arb_swap_props(),
        events in prop::collection::vec(arb_swap_event(), 0..128),
    ) {
        let mut service = Service::<utility_core::swap::Machine>::new(
            props,
            &Env::default(),
            &utility_core::swap::Messages::default(),
        );

        for event in events {
            let before_checked = *service.context().checked.get();

            let before_disabled = service.context().disabled;

            let value_event = matches!(
                event,
                utility_core::swap::Event::Toggle | utility_core::swap::Event::SetOn | utility_core::swap::Event::SetOff
            );

            drop(service.send(event));

            let state = service.state();
            let ctx = service.context();

            prop_assert_eq!(matches!(state, utility_core::swap::State::On), *ctx.checked.get());

            if before_disabled && value_event {
                prop_assert_eq!(
                    *ctx.checked.get(),
                    before_checked,
                    "disabled swap must not change checked value"
                );
            }
        }
    }
}
