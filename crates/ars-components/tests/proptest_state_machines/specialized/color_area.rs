use ars_components::specialized::color_area::{Event, Machine, Props};
use ars_core::{AriaAttr, ColorChannel, Env, HtmlAttr, Service};
use proptest::prelude::*;

fn arb_event() -> impl Strategy<Value = Event> {
    prop_oneof![
        (0.0f64..1.0, 0.0f64..1.0).prop_map(|(x, y)| Event::DragStart { x, y }),
        (0.0f64..1.0, 0.0f64..1.0).prop_map(|(x, y)| Event::DragMove { x, y }),
        Just(Event::DragEnd),
        (0.0f64..0.5).prop_map(|step| Event::IncrementX { step }),
        (0.0f64..0.5).prop_map(|step| Event::DecrementX { step }),
        (0.0f64..0.5).prop_map(|step| Event::IncrementY { step }),
        (0.0f64..0.5).prop_map(|step| Event::DecrementY { step }),
        Just(Event::SetXToMin),
        Just(Event::SetXToMax),
        Just(Event::SetYToMin),
        Just(Event::SetYToMax),
        any::<bool>().prop_map(|is_keyboard| Event::Focus { is_keyboard }),
        Just(Event::Blur),
    ]
}

proptest! {
    #![proptest_config(super::super::common::proptest_config())]

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
    fn area_event_sequences_keep_value_valid(
        events in prop::collection::vec(arb_event(), 0..64),
    ) {
        let mut svc = Service::<Machine>::new(
            Props { id: "ca".into(), ..Props::default() },
            &Env::default(),
            &Default::default(),
        );

        for ev in events {
            drop(svc.send(ev));
        }

        let api = svc.connect(&|_| {});

        let value = *api.value();

        prop_assert!((0.0..360.0).contains(&value.hue));
        prop_assert!((0.0..=1.0).contains(&value.saturation));
        prop_assert!((0.0..=1.0).contains(&value.lightness));

        let thumb = api.thumb_attrs();

        prop_assert!(
            thumb
                .get(&HtmlAttr::Aria(AriaAttr::ValueText))
                .is_some_and(|text| !text.is_empty())
        );

        for ch in [ColorChannel::Saturation, ColorChannel::Lightness] {
            let (min, max) = ars_core::channel_range(ch);

            let val = ars_core::channel_value(&value, ch);

            prop_assert!(val >= min - 1e-9 && val <= max + 1e-9);
        }
    }
}
