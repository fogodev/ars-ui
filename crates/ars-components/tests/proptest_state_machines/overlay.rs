use ars_components::overlay::presence;
use ars_core::{Env, Service};
use proptest::prelude::*;

fn arb_presence_props() -> impl Strategy<Value = presence::Props> {
    (any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
        |(present, lazy_mount, skip_animation, reduce_motion)| presence::Props {
            id: "presence".to_string(),
            present,
            lazy_mount,
            skip_animation,
            reduce_motion,
        },
    )
}

fn arb_presence_event() -> impl Strategy<Value = presence::Event> {
    prop_oneof![
        Just(presence::Event::Mount),
        Just(presence::Event::Unmount),
        Just(presence::Event::ContentReady),
        Just(presence::Event::AnimationEnd),
    ]
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
    fn proptest_presence_state_context_invariants_hold(
        props in arb_presence_props(),
        events in prop::collection::vec(arb_presence_event(), 0..128),
    ) {
        let mut service = Service::<presence::Machine>::new(props, &Env::default(), &presence::Messages);

        for event in events {
            drop(service.send(event));

            match service.state() {
                presence::State::Unmounted => {
                    prop_assert!(!service.context().present);
                    prop_assert!(!service.context().mounted);
                    prop_assert!(!service.context().unmounting);
                }

                presence::State::Mounting => {
                    prop_assert!(service.context().present);
                    prop_assert!(service.context().mounted);
                    prop_assert!(!service.context().unmounting);
                }

                presence::State::Mounted => {
                    prop_assert!(service.context().present);
                    prop_assert!(service.context().mounted);
                    prop_assert!(!service.context().unmounting);
                }

                presence::State::UnmountPending => {
                    prop_assert!(!service.context().present);
                    prop_assert!(service.context().mounted);
                    prop_assert!(service.context().unmounting);
                }
            }
        }
    }
}
