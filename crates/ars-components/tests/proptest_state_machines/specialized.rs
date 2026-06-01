//! Property-based state-machine tests for the specialized color components.
//!
//! Each block feeds an arbitrary event sequence through a machine and asserts
//! the component's core invariants always hold: stored values stay in range,
//! states stay within the declared set, and `connect()` never panics. Run in
//! the nightly `extended-proptest` job; `#[ignore]`d in the fast tier.

// ────────────────────────────────────────────────────────────────────
// ColorArea
// ────────────────────────────────────────────────────────────────────

mod color_area_proptests {
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
        #[ignore = "proptest — nightly extended-proptest job"]
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

            // connect output reflects the in-range state: the thumb always
            // exposes a non-empty `aria-valuetext` describing the current color.
            let thumb = api.thumb_attrs();

            prop_assert!(
                thumb
                    .get(&HtmlAttr::Aria(AriaAttr::ValueText))
                    .is_some_and(|text| !text.is_empty())
            );

            // The two channels stay within their ranges.
            for ch in [ColorChannel::Saturation, ColorChannel::Lightness] {
                let (min, max) = ars_core::channel_range(ch);

                let val = ars_core::channel_value(&value, ch);

                prop_assert!(val >= min - 1e-9 && val <= max + 1e-9);
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// ColorSlider
// ────────────────────────────────────────────────────────────────────

mod color_slider_proptests {
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
        #[ignore = "proptest — nightly extended-proptest job"]
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

            // The thumb's rendered `aria-valuenow` tracks the in-range value.
            let thumb = api.thumb_attrs();

            let rendered = thumb
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow))
                .expect("thumb exposes aria-valuenow")
                .parse::<f64>()
                .expect("aria-valuenow is numeric");

            prop_assert!(rendered >= min - 1e-9 && rendered <= max + 1e-9);
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// ColorWheel
// ────────────────────────────────────────────────────────────────────

mod color_wheel_proptests {
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
        #[ignore = "proptest — nightly extended-proptest job"]
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

            // The thumb's rendered `aria-valuenow` is a hue degree in [0, 360].
            let thumb = api.thumb_attrs();

            let rendered = thumb
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow))
                .expect("thumb exposes aria-valuenow")
                .parse::<f64>()
                .expect("aria-valuenow is numeric");

            prop_assert!((0.0..=360.0).contains(&rendered));
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// AngleSlider
// ────────────────────────────────────────────────────────────────────

mod angle_slider_proptests {
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
        #[ignore = "proptest — nightly extended-proptest job"]
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

            // The thumb's rendered `aria-valuenow` tracks the in-range value.
            let thumb = api.thumb_attrs();

            let rendered = thumb
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow))
                .expect("thumb exposes aria-valuenow")
                .parse::<f64>()
                .expect("aria-valuenow is numeric");

            prop_assert!(rendered >= min - 1e-9 && rendered <= max + 1e-9);
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// ColorSwatchPicker
// ────────────────────────────────────────────────────────────────────

mod color_swatch_picker_proptests {
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
        #[ignore = "proptest — nightly extended-proptest job"]
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

            // The listbox root's rendered `data-ars-state` is always one of the
            // two declared states.
            let root = api.root_attrs();

            prop_assert_eq!(root.get(&HtmlAttr::Role), Some("listbox"));

            let state = root
                .get(&HtmlAttr::Data("ars-state"))
                .expect("root exposes data-ars-state");

            prop_assert!(state == "idle" || state == "focused");
        }
    }
}
