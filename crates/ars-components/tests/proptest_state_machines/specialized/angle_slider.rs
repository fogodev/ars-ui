use ars_components::specialized::angle_slider::{Event, Machine, Props};
use ars_core::{AriaAttr, Env, HtmlAttr, KeyboardKey, Service};
use proptest::prelude::*;

fn arb_event() -> impl Strategy<Value = Event> {
    prop_oneof![
        (0.0f64..360.0).prop_map(|angle| Event::DragStart { angle }),
        (0.0f64..360.0).prop_map(|angle| Event::DragMove { angle }),
        Just(Event::DragEnd),
        Just(Event::Increment),
        Just(Event::Decrement),
        (-90.0f64..450.0).prop_map(|angle| Event::SetValue { angle }),
        any::<bool>().prop_map(|is_keyboard| Event::Focus { is_keyboard }),
        Just(Event::Blur),
        prop_oneof![
            Just(KeyboardKey::ArrowUp),
            Just(KeyboardKey::ArrowDown),
            Just(KeyboardKey::Home),
            Just(KeyboardKey::End),
            Just(KeyboardKey::PageUp),
        ]
        .prop_map(|key| Event::KeyDown { key }),
    ]
}

proptest! {
    #![proptest_config(super::super::common::proptest_config())]

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
    fn angle_event_sequences_keep_value_in_min_max(
        min in 0.0f64..180.0,
        range in 1.0f64..180.0,
        events in prop::collection::vec(arb_event(), 0..64),
    ) {
        let max = min + range;

        let mut svc = Service::<Machine>::new(
            Props { id: "as".into(), min, max, default_value: min, ..Props::default() },
            &Env::default(),
            &Default::default(),
        );

        for ev in events {
            drop(svc.send(ev));
        }

        let api = svc.connect(&|_| {});

        let value = api.value();

        prop_assert!(value >= min - 1e-9 && value <= max + 1e-9, "value {value} out of [{min}, {max}]");

        let thumb = api.thumb_attrs();

        let rendered = thumb
            .get(&HtmlAttr::Aria(AriaAttr::ValueNow))
            .expect("thumb exposes aria-valuenow")
            .parse::<f64>()
            .expect("aria-valuenow is numeric");

        prop_assert!(rendered >= min - 1e-9 && rendered <= max + 1e-9);
    }
}
