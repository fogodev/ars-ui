use ars_components::specialized::clipboard::{CopyFailureReason, Event, Machine, Props, State};
use ars_core::{ConnectApi, Env, HtmlAttr, Service};
use proptest::prelude::*;

fn arb_failure_reason() -> impl Strategy<Value = CopyFailureReason> {
    prop_oneof![
        Just(CopyFailureReason::PermissionDenied),
        Just(CopyFailureReason::NotSecureContext),
        Just(CopyFailureReason::Timeout),
        Just(CopyFailureReason::ApiUnavailable),
        "[a-z]{0,16}".prop_map(CopyFailureReason::Unknown),
    ]
}

fn arb_event() -> impl Strategy<Value = Event> {
    prop_oneof![
        Just(Event::Copy),
        Just(Event::CopySuccess),
        arb_failure_reason().prop_map(Event::CopyError),
        Just(Event::ResetTimeout),
        proptest::option::of("[a-zA-Z0-9 _-]{0,24}".prop_map(String::from))
            .prop_map(Event::SetValue),
        Just(Event::SetProps),
    ]
}

proptest! {
    #![proptest_config(super::super::common::proptest_config())]

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
    fn clipboard_event_sequences_preserve_state_and_attr_invariants(
        default_value in "[a-zA-Z0-9 _-]{0,24}",
        events in prop::collection::vec(arb_event(), 0..96),
    ) {
        let props = Props::new().id("clip").default_value(default_value);

        let mut svc = Service::<Machine>::new(props, &Env::default(), &Default::default());

        for ev in events {
            let mut result = svc.send(ev);

            result.pending_effects.clear();
        }

        let state = *svc.state();

        if state != State::Error {
            prop_assert_eq!(svc.context().error.as_ref(), None);
        }

        let api = svc.connect(&|_| {});

        let expected_state = state.to_string();

        let root = api.root_attrs();
        let trigger = api.trigger_attrs();

        prop_assert_eq!(
            root.get(&HtmlAttr::Data("ars-state")),
            Some(expected_state.as_str())
        );
        prop_assert_eq!(
            trigger.get(&HtmlAttr::Data("ars-state")),
            Some(expected_state.as_str())
        );
        prop_assert_eq!(api.part_attrs(ars_components::specialized::clipboard::Part::Root), root);
        prop_assert_eq!(api.part_attrs(ars_components::specialized::clipboard::Part::Trigger), trigger);
        prop_assert_eq!(api.part_attrs(ars_components::specialized::clipboard::Part::Status), api.status_attrs());
    }
}
