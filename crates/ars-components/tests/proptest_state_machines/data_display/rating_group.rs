use std::num::NonZero;

use ars_components::data_display::rating_group;
use ars_core::{Env, Service};
use proptest::prelude::*;

fn arb_props() -> impl Strategy<Value = rating_group::Props> {
    (
        1u32..=8,
        0.0f64..=8.0,
        any::<bool>(),
        prop_oneof![Just(0.25), Just(0.5), Just(1.0), Just(2.0)],
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(count, default_value, allow_half, step, readonly, disabled)| {
                rating_group::Props::new()
                    .id("rating")
                    .count(NonZero::new(count).expect("non-zero"))
                    .default_value(default_value)
                    .allow_half(allow_half)
                    .step(step)
                    .readonly(readonly)
                    .disabled(disabled)
            },
        )
}

fn arb_event() -> impl Strategy<Value = rating_group::Event> {
    prop_oneof![
        (-4.0f64..=12.0f64).prop_map(rating_group::Event::Rate),
        (0usize..8).prop_map(rating_group::Event::HoverItem),
        (-4.0f64..=12.0f64).prop_map(rating_group::Event::HoverValue),
        Just(rating_group::Event::UnHover),
        (0usize..8, any::<bool>())
            .prop_map(|(index, is_keyboard)| { rating_group::Event::Focus { index, is_keyboard } }),
        Just(rating_group::Event::Blur),
        Just(rating_group::Event::IncrementRating),
        Just(rating_group::Event::DecrementRating),
        Just(rating_group::Event::ClearRating),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_rating_group_event_sequences_preserve_invariants(
        props in arb_props(),
        events in prop::collection::vec(arb_event(), 0..64),
    ) {
        let mut service = Service::<rating_group::Machine>::new(
            props,
            &Env::default(),
            &rating_group::Messages::default(),
        );

        let initial_value = *service.context().value.get();

        for event in events {
            drop(service.send(event));

            let ctx = service.context();
            let value = *ctx.value.get();

            let max = f64::from(ctx.count.get());

            prop_assert!(value.is_finite());
            prop_assert!((0.0..=max).contains(&value));

            if let Some(hovered) = ctx.hovered_value {
                prop_assert!(hovered.is_finite());
                prop_assert!((0.0..=max).contains(&hovered));
            }

            if ctx.disabled || ctx.readonly {
                prop_assert_eq!(value, initial_value);
            }
        }
    }
}
