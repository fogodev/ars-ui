use ars_components::specialized::color_field::{Event, Machine, Props, State};
use ars_core::{
    AriaAttr, ColorChannel, ColorFormat, ColorValue, ConnectApi, Env, HtmlAttr, Service,
};
use proptest::prelude::*;

fn arb_channel() -> impl Strategy<Value = Option<ColorChannel>> {
    prop_oneof![
        Just(None),
        Just(Some(ColorChannel::Hue)),
        Just(Some(ColorChannel::Saturation)),
        Just(Some(ColorChannel::Lightness)),
        Just(Some(ColorChannel::Alpha)),
        Just(Some(ColorChannel::Red)),
        Just(Some(ColorChannel::Green)),
        Just(Some(ColorChannel::Blue)),
    ]
}

fn arb_event() -> impl Strategy<Value = Event> {
    prop_oneof![
        any::<bool>().prop_map(|is_keyboard| Event::Focus { is_keyboard }),
        Just(Event::Blur),
        "[#a-zA-Z0-9(),.% -]{0,32}".prop_map(Event::Change),
        Just(Event::Commit),
        Just(Event::SetValue(ColorValue::from_rgb(255, 0, 0))),
        any::<bool>().prop_map(Event::SetInvalid),
        any::<bool>().prop_map(Event::SetHasDescription),
        Just(Event::Increment),
        Just(Event::Decrement),
        Just(Event::IncrementLarge),
        Just(Event::DecrementLarge),
        Just(Event::IncrementToMax),
        Just(Event::DecrementToMin),
        Just(Event::CompositionStart),
        Just(Event::CompositionEnd),
    ]
}

proptest! {
    #![proptest_config(super::super::common::proptest_config())]

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
    fn color_field_event_sequences_preserve_value_and_attr_invariants(
        channel in arb_channel(),
        disabled in any::<bool>(),
        readonly in any::<bool>(),
        required in any::<bool>(),
        events in prop::collection::vec(arb_event(), 0..96),
    ) {
        let props = Props {
            id: "field".into(),
            default_value: Some(ColorValue::default()),
            channel,
            color_format: ColorFormat::Hex,
            disabled,
            readonly,
            required,
            name: Some("color".into()),
            ..Props::default()
        };
        let mut svc = Service::<Machine>::new(props, &Env::default(), &Default::default());

        for ev in events {
            drop(svc.send(ev));
        }

        match svc.state() {
            State::Idle | State::Focused => {}
        }

        if let Some(value) = svc.context().value.pending() {
            prop_assert!(value.hue.is_finite() && (0.0..360.0).contains(&value.hue));
            prop_assert!(value.saturation.is_finite() && (0.0..=1.0).contains(&value.saturation));
            prop_assert!(value.lightness.is_finite() && (0.0..=1.0).contains(&value.lightness));
            prop_assert!(value.alpha.is_finite() && (0.0..=1.0).contains(&value.alpha));
        }

        let api = svc.connect(&|_| {});

        let input = api.input_attrs();

        if channel.is_some() {
            prop_assert_eq!(input.get(&HtmlAttr::Role), Some("spinbutton"));
            prop_assert!(
                input
                    .get(&HtmlAttr::Aria(AriaAttr::ValueNow))
                    .is_some_and(|value| value.parse::<f64>().is_ok())
            );
        } else {
            prop_assert_eq!(input.get(&HtmlAttr::InputMode), Some("text"));
        }

        let hidden = api.hidden_input_attrs();

        if disabled || api.is_invalid() {
            prop_assert!(hidden.contains(&HtmlAttr::Disabled));
        }

        prop_assert_eq!(api.part_attrs(ars_components::specialized::color_field::Part::Input), api.input_attrs());
        prop_assert_eq!(api.part_attrs(ars_components::specialized::color_field::Part::HiddenInput), api.hidden_input_attrs());
    }
}
