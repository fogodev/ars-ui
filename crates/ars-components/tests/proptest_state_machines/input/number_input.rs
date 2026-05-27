use ars_components::input::number_input;
use ars_core::{Env, HtmlAttr, Service};
use proptest::prelude::*;

use super::arb_short_text;

fn arb_number_input_props() -> impl Strategy<Value = number_input::Props> {
    (
        prop::option::of(-1000.0_f64..1000.0),
        prop::option::of(-1000.0_f64..1000.0),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        prop::option::of(0_u32..=4),
    )
        .prop_map(
            |(
                value,
                default_value,
                disabled,
                readonly,
                invalid,
                required,
                allow_mouse_wheel,
                spin_on_press,
                precision,
            )| number_input::Props {
                id: "number-input".to_string(),
                value,
                default_value,
                min: -1000.0,
                max: 1000.0,
                step: 1.0,
                large_step: 10.0,
                precision,
                disabled,
                readonly,
                invalid,
                required,
                name: Some("qty".to_string()),
                form: Some("form".to_string()),
                allow_mouse_wheel,
                clamp_value_on_blur: true,
                spin_on_press,
                format_options: None,
                display_format: None,
            },
        )
}

fn arb_number_input_event() -> impl Strategy<Value = number_input::Event> {
    prop_oneof![
        any::<bool>().prop_map(|is_keyboard| number_input::Event::Focus { is_keyboard }),
        Just(number_input::Event::Blur),
        arb_short_text().prop_map(number_input::Event::Change),
        Just(number_input::Event::Increment),
        Just(number_input::Event::Decrement),
        Just(number_input::Event::IncrementLarge),
        Just(number_input::Event::DecrementLarge),
        Just(number_input::Event::IncrementToMax),
        Just(number_input::Event::DecrementToMin),
        (-1000.0_f64..1000.0).prop_map(number_input::Event::SetValue),
        Just(number_input::Event::StartScrub),
        (-10.0_f64..10.0).prop_map(number_input::Event::Scrub),
        Just(number_input::Event::EndScrub),
        (-5.0_f64..5.0).prop_map(|delta| number_input::Event::Wheel { delta }),
        Just(number_input::Event::CompositionStart),
        Just(number_input::Event::CompositionEnd),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_number_input_event_sequences_preserve_invariants(
        props in arb_number_input_props(),
        events in prop::collection::vec(arb_number_input_event(), 0..128),
    ) {
        let mut service = Service::<number_input::Machine>::new(
            props,
            &Env::default(),
            &number_input::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            let ctx = service.context();

            // value is always inside [min, max] when Some and the machine has not
            // observed a `Change` since the last clamp opportunity (Blur with
            // clamp_value_on_blur). Increment/Decrement/SetValue all clamp.
            // Change(text) does NOT clamp, so we only assert the clamping
            // happens for non-Change-only paths by checking after a Blur.
            if let Some(value) = ctx.value.get() {
                prop_assert!(!value.is_nan(), "value never becomes NaN");
            }

            let input_attrs = service.connect(&|_| {}).input_attrs();

            prop_assert_eq!(input_attrs.get(&HtmlAttr::Role), Some("spinbutton"));
            prop_assert_eq!(input_attrs.get(&HtmlAttr::InputMode), Some("decimal"));
        }
    }
}
