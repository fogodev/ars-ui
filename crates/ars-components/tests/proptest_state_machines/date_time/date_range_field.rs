use ars_components::date_time::date_range_field;
use ars_core::{ComponentPart, ConnectApi, Env, Service};
use ars_i18n::DateRange;
use proptest::prelude::*;

use super::helpers::arb_date;

fn arb_date_range_field_event() -> impl Strategy<Value = date_range_field::Event> {
    prop_oneof![
        Just(date_range_field::Event::FocusStart),
        Just(date_range_field::Event::FocusEnd),
        Just(date_range_field::Event::BlurAll),
        proptest::option::of(arb_date()).prop_map(date_range_field::Event::StartValueChange),
        proptest::option::of(arb_date()).prop_map(date_range_field::Event::EndValueChange),
        proptest::option::of((arb_date(), arb_date()).prop_map(|(first, second)| {
            DateRange::normalized(first, second).expect("comparable generated dates")
        }))
        .prop_map(date_range_field::Event::SetRange),
    ]
}

fn assert_date_range_field_invariants(service: &Service<date_range_field::Machine>) {
    let ctx = service.context();

    // The derived complete range is `Some` exactly when both fields are set,
    // and is always normalized so `start <= end`.
    if let Some(range) = ctx.value.get() {
        assert!(
            ctx.start_date.is_some() && ctx.end_date.is_some(),
            "complete range requires both fields set"
        );
        assert!(
            matches!(
                range.start.compare_within_calendar(&range.end),
                Some(core::cmp::Ordering::Less | core::cmp::Ordering::Equal)
            ),
            "stored range must be normalized so start <= end"
        );
        assert_eq!(ctx.start_date.as_ref(), Some(&range.start));
        assert_eq!(ctx.end_date.as_ref(), Some(&range.end));
    } else {
        assert!(
            ctx.start_date.is_none() || ctx.end_date.is_none(),
            "an empty range must have at least one unset field"
        );
    }

    // Every part produces attributes without panicking.
    let send = |_event: date_range_field::Event| {};

    let api = service.connect(&send);

    for part in date_range_field::Part::all() {
        drop(api.part_attrs(part));
    }

    // Child field props mirror the tracked per-field values.
    assert_eq!(api.start_field_props().value, Some(ctx.start_date.clone()));
    assert_eq!(api.end_field_props().value, Some(ctx.end_date.clone()));
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    fn date_range_field_maintains_normalized_range(
        events in prop::collection::vec(arb_date_range_field_event(), 0..24),
    ) {
        let mut service = Service::<date_range_field::Machine>::new(
            date_range_field::Props::new().id("range"),
            &Env::default(),
            &date_range_field::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            assert_date_range_field_invariants(&service);
        }
    }

    #[test]
    fn date_range_field_disabled_ignores_events(
        events in prop::collection::vec(arb_date_range_field_event(), 0..24),
    ) {
        let mut service = Service::<date_range_field::Machine>::new(
            date_range_field::Props::new().id("range").disabled(true),
            &Env::default(),
            &date_range_field::Messages::default(),
        );

        for event in events {
            drop(service.send(event));
        }

        prop_assert_eq!(service.state(), &date_range_field::State::Idle);
        prop_assert_eq!(service.context().value.get().as_ref(), None);
    }
}
