use ars_components::specialized::color_picker::{Event, Machine, Props};
use ars_core::{ColorChannel, ColorFormat, ColorSpace, ColorValue, Env, Service};
use proptest::prelude::*;

fn arb_channel() -> impl Strategy<Value = ColorChannel> {
    prop_oneof![
        Just(ColorChannel::Hue),
        Just(ColorChannel::Saturation),
        Just(ColorChannel::Lightness),
        Just(ColorChannel::Brightness),
        Just(ColorChannel::Alpha),
        Just(ColorChannel::Red),
        Just(ColorChannel::Green),
        Just(ColorChannel::Blue),
    ]
}

fn arb_target() -> impl Strategy<Value = ars_core::DragTarget> {
    prop_oneof![
        Just(ars_core::DragTarget::Area),
        arb_channel().prop_map(ars_core::DragTarget::Channel),
    ]
}

fn arb_event() -> impl Strategy<Value = Event> {
    prop_oneof![
        Just(Event::Open),
        Just(Event::Close),
        Just(Event::Toggle),
        (arb_target(), 0.0f64..1.0, 0.0f64..1.0).prop_map(|(target, x, y)| Event::DragStart {
            target,
            x,
            y
        }),
        (0.0f64..1.0, 0.0f64..1.0).prop_map(|(x, y)| Event::DragMove { x, y }),
        Just(Event::DragEnd),
        (-50.0f64..400.0, -50.0f64..400.0, -1.0f64..2.0)
            .prop_map(|(h, s, l)| Event::SetColor(ColorValue::new(h, s, l, 1.0))),
        (arb_channel(), -50.0f64..400.0)
            .prop_map(|(channel, value)| Event::SetChannel { channel, value }),
        (arb_channel(), 0.0f64..50.0)
            .prop_map(|(channel, step)| Event::ChannelIncrement { channel, step }),
        (arb_channel(), 0.0f64..50.0)
            .prop_map(|(channel, step)| Event::ChannelDecrement { channel, step }),
        Just(Event::SetFormat(ColorFormat::Rgb)),
        Just(Event::ChangeColorSpace(ColorSpace::Hsb)),
        any::<bool>().prop_map(Event::SetEyedropperSupported),
        Just(Event::EyedropperRequest),
        Just(Event::CloseOnEscape),
        Just(Event::CloseOnInteractOutside),
    ]
}

proptest! {
    #![proptest_config(super::super::common::proptest_config())]

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
    fn picker_event_sequences_keep_color_valid(
        events in prop::collection::vec(arb_event(), 0..64),
    ) {
        let mut svc = Service::<Machine>::new(
            Props { id: "cp".into(), ..Props::default() },
            &Env::default(),
            &Default::default(),
        );

        for ev in events {
            let mut result = svc.send(ev);

            result.pending_effects.clear();
        }

        match svc.state() {
            ars_components::specialized::color_picker::State::Closed
            | ars_components::specialized::color_picker::State::Open
            | ars_components::specialized::color_picker::State::Dragging { .. } => {}
        }

        let api = svc.connect(&|_| {});

        let value = *api.value();

        prop_assert!(value.hue.is_finite() && (0.0..360.0).contains(&value.hue));
        prop_assert!(value.saturation.is_finite() && (0.0..=1.0).contains(&value.saturation));
        prop_assert!(value.lightness.is_finite() && (0.0..=1.0).contains(&value.lightness));
        prop_assert!(value.alpha.is_finite() && (0.0..=1.0).contains(&value.alpha));

        prop_assert!(!api.value_as_string().is_empty());

        drop(api.area_thumb_attrs());
        drop(api.hidden_input_attrs());
    }
}
