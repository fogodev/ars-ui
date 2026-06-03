use ars_components::data_display::progress;
use ars_core::{AriaAttr, Env, HtmlAttr, Service};
use proptest::prelude::*;

fn arb_props() -> impl Strategy<Value = progress::Props> {
    (
        prop::option::of(prop::option::of(-100.0f64..=200.0)),
        prop::option::of(-100.0f64..=200.0),
        -100.0f64..=50.0,
        51.0f64..=250.0,
        prop_oneof![
            Just(progress::Orientation::Horizontal),
            Just(progress::Orientation::Vertical),
        ],
    )
        .prop_map(
            |(value, default_value, min, max, orientation)| progress::Props {
                id: "progress".to_string(),
                value,
                default_value,
                min,
                max,
                orientation,
                format_options: None,
            },
        )
}

fn arb_event() -> impl Strategy<Value = progress::Event> {
    prop_oneof![
        prop::option::of(-100.0f64..=250.0).prop_map(progress::Event::SetValue),
        (51.0f64..=300.0f64).prop_map(progress::Event::SetMax),
        Just(progress::Event::Complete),
        Just(progress::Event::Reset),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_progress_event_sequences_preserve_invariants(
        props in arb_props(),
        events in prop::collection::vec(arb_event(), 0..64),
    ) {
        let mut service = Service::<progress::Machine>::new(
            props,
            &Env::default(),
            &progress::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            let ctx = service.context();

            prop_assert!((0.0..=100.0).contains(&ctx.percent));
            prop_assert_eq!(ctx.indeterminate, ctx.value.get().is_none());

            match ctx.value.get() {
                None => {
                    prop_assert!(
                        matches!(service.state(), progress::State::Loading),
                        "indeterminate value must report loading state"
                    );

                    let attrs = service.connect(&|_| {}).root_attrs();

                    prop_assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::ValueNow)));
                }

                Some(value) if *value >= ctx.max => {
                    prop_assert_eq!(service.state(), &progress::State::Complete);
                    prop_assert_eq!(ctx.percent, 100.0);
                }

                Some(_) => {
                    prop_assert_eq!(service.state(), &progress::State::Idle);
                }
            }
        }
    }
}
