use ars_components::specialized::color_slider::{Event, Machine, Props};
use ars_core::{AriaAttr, ColorChannel, Env, HtmlAttr, Service};
use proptest::prelude::*;

fn arb_channel() -> impl Strategy<Value = ColorChannel> {
    prop_oneof![
        Just(ColorChannel::Hue),
        Just(ColorChannel::Saturation),
        Just(ColorChannel::Alpha),
        Just(ColorChannel::Red),
    ]
}

fn arb_event() -> impl Strategy<Value = Event> {
    prop_oneof![
        (0.0f64..1.0).prop_map(|position| Event::DragStart { position }),
        (0.0f64..1.0).prop_map(|position| Event::DragMove { position }),
        Just(Event::DragEnd),
        (0.0f64..50.0).prop_map(|step| Event::Increment { step }),
        (0.0f64..50.0).prop_map(|step| Event::Decrement { step }),
        Just(Event::SetToMin),
        Just(Event::SetToMax),
        any::<bool>().prop_map(|is_keyboard| Event::Focus { is_keyboard }),
        Just(Event::Blur),
    ]
}

proptest! {
    #![proptest_config(super::super::common::proptest_config())]

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
    fn slider_event_sequences_keep_channel_in_range(
        channel in arb_channel(),
        events in prop::collection::vec(arb_event(), 0..64),
    ) {
        let mut svc = Service::<Machine>::new(
            Props { id: "cs".into(), channel, ..Props::default() },
            &Env::default(),
            &Default::default(),
        );

        for ev in events {
            drop(svc.send(ev));
        }

        let api = svc.connect(&|_| {});

        let (min, max) = ars_core::channel_range(channel);
        let val = ars_core::channel_value(api.value(), channel);

        prop_assert!(val >= min - 1e-9 && val <= max + 1e-9);

        let thumb = api.thumb_attrs();

        let rendered = thumb
            .get(&HtmlAttr::Aria(AriaAttr::ValueNow))
            .expect("thumb exposes aria-valuenow")
            .parse::<f64>()
            .expect("aria-valuenow is numeric");

        prop_assert!(rendered >= min - 1e-9 && rendered <= max + 1e-9);
    }
}
