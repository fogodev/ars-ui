use ars_components::specialized::color_wheel::{Event, Machine, Props};
use ars_core::{AriaAttr, Env, HtmlAttr, Service};
use proptest::prelude::*;

fn arb_event() -> impl Strategy<Value = Event> {
    prop_oneof![
        (0.0f64..1.0).prop_map(|position| Event::DragStart { position }),
        (0.0f64..1.0).prop_map(|position| Event::DragMove { position }),
        Just(Event::DragEnd),
        (0.0f64..90.0).prop_map(|step| Event::Increment { step }),
        (0.0f64..90.0).prop_map(|step| Event::Decrement { step }),
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
    fn wheel_event_sequences_keep_hue_in_range(
        events in prop::collection::vec(arb_event(), 0..64),
    ) {
        let mut svc = Service::<Machine>::new(
            Props { id: "cw".into(), ..Props::default() },
            &Env::default(),
            &Default::default(),
        );

        for ev in events {
            drop(svc.send(ev));
        }

        let api = svc.connect(&|_| {});

        prop_assert!((0.0..=360.0).contains(&api.value().hue));

        let thumb = api.thumb_attrs();

        let rendered = thumb
            .get(&HtmlAttr::Aria(AriaAttr::ValueNow))
            .expect("thumb exposes aria-valuenow")
            .parse::<f64>()
            .expect("aria-valuenow is numeric");

        prop_assert!((0.0..=360.0).contains(&rendered));
    }
}
