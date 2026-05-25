use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_fieldset_event_sequences_preserve_invariants(
        props in arb_fieldset_props(),
        events in prop::collection::vec(arb_fieldset_event(), 0..128),
    ) {
        let mut service = Service::<utility_core::fieldset::Machine>::new(props, &Env::default(), &());

        for event in events {
            drop(service.send(event));

            prop_assert_eq!(service.state(), &utility_core::fieldset::State::Idle);
            prop_assert_eq!(service.context().ids.id(), "fieldset");
            prop_assert!(service.context().errors.is_empty() || service.context().invalid);
        }
    }
}
