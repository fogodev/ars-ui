use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_form_submit_event_sequences_preserve_invariants(
        initial_mode in arb_mode(),
        events in prop::collection::vec(arb_form_submit_event(), 0..128),
    ) {
        let mut service = Service::<utility_core::form_submit::Machine>::new(
            form_submit_props(initial_mode),
            &Env::default(),
            &(),
        );

        for event in events {
            drop(service.send(event));

            prop_assert_eq!(
                service.context().form.is_submitting,
                matches!(service.state(), utility_core::form_submit::State::Submitting)
            );
            prop_assert_eq!(
                service.context().submit_error.is_some(),
                matches!(service.state(), utility_core::form_submit::State::Failed)
            );
            prop_assert_eq!(service.context().ids.id(), "test-form");
        }
    }
}
