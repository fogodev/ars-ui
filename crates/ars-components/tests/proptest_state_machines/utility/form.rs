use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_form_event_sequences_preserve_invariants(
        props in arb_form_props(),
        events in prop::collection::vec(arb_form_event(), 0..128),
    ) {
        let mut service = Service::<utility_core::form::Machine>::new(props, &Env::default(), &());

        for event in events {
            drop(service.send(event));

            prop_assert_eq!(
                service.context().is_submitting,
                matches!(service.state(), utility_core::form::State::Submitting)
            );
            prop_assert_eq!(service.context().ids.id(), "form");
        }
    }
}
