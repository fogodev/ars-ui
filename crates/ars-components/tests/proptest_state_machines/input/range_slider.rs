use ars_components::input::{range_slider, slider};
use ars_core::{AriaAttr, Direction, Env, HtmlAttr, Orientation, Service};
use proptest::prelude::*;

use super::arb_slider_step;

fn arb_range_slider_values() -> impl Strategy<Value = [f64; 2]> {
    (0.0_f64..100.0, 0.0_f64..100.0).prop_map(|(start, end)| [start, end])
}

fn arb_range_slider_props() -> impl Strategy<Value = range_slider::Props> {
    (
        prop::option::of(arb_range_slider_values()),
        arb_range_slider_values(),
        arb_slider_step(),
        0_u32..=4,
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(
                value,
                default_value,
                step,
                min_steps_between,
                disabled,
                readonly,
                invalid,
                vertical,
                rtl,
                allow_thumb_swap,
                start_disabled,
                end_disabled,
            )| range_slider::Props {
                id: "range-slider".to_string(),
                value,
                default_value,
                min: 0.0,
                max: 100.0,
                step,
                large_step: Some(step * 10.0),
                min_steps_between,
                disabled,
                readonly,
                invalid,
                orientation: if vertical {
                    Orientation::Vertical
                } else {
                    Orientation::Horizontal
                },
                dir: if rtl { Direction::Rtl } else { Direction::Ltr },
                name: Some("price".to_string()),
                form: Some("form".to_string()),
                allow_thumb_swap,
                start_disabled,
                end_disabled,
                format_value: None,
                thumb_alignment: slider::ThumbAlignment::Contain,
                on_value_change: None,
                on_value_change_end: None,
            },
        )
}

fn arb_range_slider_thumb() -> impl Strategy<Value = range_slider::ThumbIndex> {
    prop_oneof![
        Just(range_slider::ThumbIndex::Start),
        Just(range_slider::ThumbIndex::End),
    ]
}

fn arb_range_slider_event() -> impl Strategy<Value = range_slider::Event> {
    prop_oneof![
        (arb_range_slider_thumb(), any::<bool>())
            .prop_map(|(thumb, is_keyboard)| { range_slider::Event::Focus { thumb, is_keyboard } }),
        arb_range_slider_thumb().prop_map(|thumb| range_slider::Event::Blur { thumb }),
        (arb_range_slider_thumb(), 0.0_f64..100.0)
            .prop_map(|(thumb, value)| { range_slider::Event::PointerDown { thumb, value } }),
        (0.0_f64..100.0).prop_map(|value| range_slider::Event::PointerMove { value }),
        Just(range_slider::Event::PointerUp),
        arb_range_slider_thumb().prop_map(|thumb| range_slider::Event::Increment { thumb }),
        arb_range_slider_thumb().prop_map(|thumb| range_slider::Event::Decrement { thumb }),
        arb_range_slider_thumb().prop_map(|thumb| range_slider::Event::IncrementLarge { thumb }),
        arb_range_slider_thumb().prop_map(|thumb| range_slider::Event::DecrementLarge { thumb }),
        arb_range_slider_thumb().prop_map(|thumb| range_slider::Event::SetToMin { thumb }),
        arb_range_slider_thumb().prop_map(|thumb| range_slider::Event::SetToMax { thumb }),
        arb_range_slider_values().prop_map(range_slider::Event::SetValues),
        prop::option::of(arb_range_slider_values()).prop_map(range_slider::Event::SyncValue),
        Just(range_slider::Event::SetProps),
        any::<bool>().prop_map(range_slider::Event::SetHasDescription),
        any::<bool>().prop_map(range_slider::Event::SetHasLabel),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_range_slider_event_sequences_preserve_invariants(
        props in arb_range_slider_props(),
        events in prop::collection::vec(arb_range_slider_event(), 0..128),
    ) {
        let mut service = Service::<range_slider::Machine>::new(
            props,
            &Env::default(),
            &range_slider::Messages::default(),
        );

        for event in events {
            let previous = *service.context().value.get();

            let interactive_mutation = matches!(
                event,
                range_slider::Event::PointerDown { .. }
                    | range_slider::Event::PointerMove { .. }
                    | range_slider::Event::Increment { .. }
                    | range_slider::Event::Decrement { .. }
                    | range_slider::Event::IncrementLarge { .. }
                    | range_slider::Event::DecrementLarge { .. }
                    | range_slider::Event::SetToMin { .. }
                    | range_slider::Event::SetToMax { .. }
                    | range_slider::Event::SetValues(_)
            );

            drop(service.send(event));

            let ctx = service.context();

            let value = *ctx.value.get();
            let min = ctx.min.min(ctx.max);
            let max = ctx.min.max(ctx.max);

            prop_assert!(value[0].is_finite());
            prop_assert!(value[1].is_finite());
            prop_assert!(value[0] >= min - f64::EPSILON);
            prop_assert!(value[1] <= max + f64::EPSILON);
            prop_assert!(value[0] <= value[1] + f64::EPSILON);

            for thumb_value in value {
                let steps = (thumb_value - min) / ctx.step;

                prop_assert!(
                    (steps - steps.round()).abs() <= 1.0e-9,
                    "value {thumb_value} must stay on step grid {min}..{max} step {}",
                    ctx.step
                );
            }

            let gap = ctx.step * f64::from(ctx.min_steps_between);

            if gap.is_finite() && max - min >= gap {
                prop_assert!(
                    value[1] - value[0] + f64::EPSILON >= gap,
                    "range {:?} must preserve gap {gap}",
                    value
                );
            }

            if (ctx.disabled || ctx.readonly) && interactive_mutation {
                prop_assert_eq!(value, previous);
            }

            let api = service.connect(&|_| {});

            let start = api.thumb_attrs(range_slider::ThumbIndex::Start);
            let end = api.thumb_attrs(range_slider::ThumbIndex::End);

            prop_assert_eq!(start.get(&HtmlAttr::Role), Some("slider"));
            prop_assert_eq!(end.get(&HtmlAttr::Role), Some("slider"));

            let start_max = start
                .get(&HtmlAttr::Aria(AriaAttr::ValueMax))
                .and_then(|value| value.parse::<f64>().ok())
                .expect("start aria-valuemax must parse");

            let end_min = end
                .get(&HtmlAttr::Aria(AriaAttr::ValueMin))
                .and_then(|value| value.parse::<f64>().ok())
                .expect("end aria-valuemin must parse");

            prop_assert!(start_max <= value[1] + f64::EPSILON);
            prop_assert!(end_min >= value[0] - f64::EPSILON);
        }
    }
}
