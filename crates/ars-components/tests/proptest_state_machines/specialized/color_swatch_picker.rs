use ars_components::specialized::color_swatch_picker::{Event, Machine, Props};
use ars_core::{ColorValue, Env, HtmlAttr, Service};
use proptest::prelude::*;

fn arb_event() -> impl Strategy<Value = Event> {
    prop_oneof![
        any::<bool>().prop_map(|is_keyboard| Event::Focus { is_keyboard }),
        Just(Event::Blur),
        (0usize..12).prop_map(|index| Event::Select { index }),
        Just(Event::FocusNext),
        Just(Event::FocusPrev),
        Just(Event::FocusUp),
        Just(Event::FocusDown),
        Just(Event::FocusFirst),
        Just(Event::FocusLast),
    ]
}

proptest! {
    #![proptest_config(super::super::common::proptest_config())]

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
    fn picker_focus_index_stays_in_bounds(
        count in 1usize..8,
        columns in 1usize..5,
        events in prop::collection::vec(arb_event(), 0..64),
    ) {
        let colors: Vec<ColorValue> = (0..count)
            .map(|index| ColorValue::from_hsl((index as f64) * 30.0, 1.0, 0.5))
            .collect();

        let mut svc = Service::<Machine>::new(
            Props { id: "csp".into(), colors, columns, ..Props::default() },
            &Env::default(),
            &Default::default(),
        );

        for ev in events {
            drop(svc.send(ev));
        }

        let api = svc.connect(&|_| {});

        if let Some(idx) = api.focused_index() {
            prop_assert!(idx < count, "focused index {idx} >= {count}");
        }

        let root = api.root_attrs();

        prop_assert_eq!(root.get(&HtmlAttr::Role), Some("listbox"));

        let state = root
            .get(&HtmlAttr::Data("ars-state"))
            .expect("root exposes data-ars-state");

        prop_assert!(state == "idle" || state == "focused");
    }
}
