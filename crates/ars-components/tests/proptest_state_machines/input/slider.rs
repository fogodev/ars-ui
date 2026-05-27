use ars_components::input::slider;
use ars_core::{AriaAttr, Direction, Env, HtmlAttr, Orientation, Service};
use proptest::prelude::*;

use super::arb_slider_step;

fn arb_slider_props() -> impl Strategy<Value = slider::Props> {
    (
        prop::option::of(0.0_f64..100.0),
        0.0_f64..100.0,
        arb_slider_step(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(value, default_value, step, disabled, readonly, invalid, vertical, rtl, discrete)| {
                slider::Props {
                    id: "slider".to_string(),
                    value,
                    default_value,
                    min: 0.0,
                    max: 100.0,
                    step,
                    large_step: Some(step * 10.0),
                    disabled,
                    readonly,
                    invalid,
                    orientation: if vertical {
                        Orientation::Vertical
                    } else {
                        Orientation::Horizontal
                    },
                    dir: if rtl { Direction::Rtl } else { Direction::Ltr },
                    origin: slider::Origin::Start,
                    name: Some("volume".to_string()),
                    form: Some("form".to_string()),
                    marks: Vec::new(),
                    tick_format: None,
                    value_format: None,
                    format_value: None,
                    format_value_text: None,
                    discrete,
                    value_labels: discrete
                        .then(|| vec!["Low".to_string(), "Medium".to_string(), "High".to_string()]),
                    thumb_alignment: slider::ThumbAlignment::Contain,
                    on_value_change: None,
                    on_value_change_end: None,
                }
            },
        )
}

fn arb_slider_event() -> impl Strategy<Value = slider::Event> {
    prop_oneof![
        any::<bool>().prop_map(|is_keyboard| slider::Event::Focus { is_keyboard }),
        Just(slider::Event::Blur),
        (0.0_f64..100.0).prop_map(|value| slider::Event::PointerDown { value }),
        (0.0_f64..100.0).prop_map(|value| slider::Event::PointerMove { value }),
        Just(slider::Event::PointerUp),
        Just(slider::Event::Increment),
        Just(slider::Event::Decrement),
        Just(slider::Event::IncrementLarge),
        Just(slider::Event::DecrementLarge),
        Just(slider::Event::SetToMin),
        Just(slider::Event::SetToMax),
        (0.0_f64..100.0).prop_map(slider::Event::SetValue),
        prop::option::of(0.0_f64..100.0).prop_map(slider::Event::SyncValue),
        Just(slider::Event::SetProps),
        any::<bool>().prop_map(slider::Event::SetHasDescription),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_slider_event_sequences_preserve_invariants(
        props in arb_slider_props(),
        events in prop::collection::vec(arb_slider_event(), 0..128),
    ) {
        let mut service = Service::<slider::Machine>::new(
            props,
            &Env::default(),
            &slider::Messages::default(),
        );

        for event in events {
            let previous = *service.context().value.get();

            let interactive_mutation = matches!(
                event,
                slider::Event::PointerDown { .. }
                    | slider::Event::PointerMove { .. }
                    | slider::Event::Increment
                    | slider::Event::Decrement
                    | slider::Event::IncrementLarge
                    | slider::Event::DecrementLarge
                    | slider::Event::SetToMin
                    | slider::Event::SetToMax
                    | slider::Event::SetValue(_)
            );
            drop(service.send(event));

            let ctx = service.context();
            let value = *ctx.value.get();

            prop_assert!(value.is_finite());
            prop_assert!(value >= ctx.min - f64::EPSILON);
            prop_assert!(value <= ctx.max + f64::EPSILON);

            let steps = (value - ctx.min) / ctx.step;

            prop_assert!(
                (steps - steps.round()).abs() <= 1.0e-9,
                "value {value} must stay on step grid {}..{} step {}",
                ctx.min,
                ctx.max,
                ctx.step
            );

            if (ctx.disabled || ctx.readonly) && interactive_mutation {
                prop_assert_eq!(value, previous);
            }

            let thumb_attrs = service.connect(&|_| {}).thumb_attrs();

            prop_assert_eq!(thumb_attrs.get(&HtmlAttr::Role), Some("slider"));

            let aria_value = thumb_attrs
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow))
                .and_then(|value| value.parse::<f64>().ok())
                .expect("aria-valuenow must parse");

            prop_assert!(aria_value >= ctx.min - f64::EPSILON);
            prop_assert!(aria_value <= ctx.max + f64::EPSILON);
        }
    }
}
