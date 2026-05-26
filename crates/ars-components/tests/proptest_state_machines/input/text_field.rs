use ars_components::input::text_field;
use ars_core::{AriaAttr, Direction, Env, HtmlAttr, InputMode, Service};
use proptest::prelude::*;

use super::arb_short_text;

fn arb_text_field_input_type() -> impl Strategy<Value = text_field::InputType> {
    prop_oneof![
        Just(text_field::InputType::Text),
        Just(text_field::InputType::Password),
        Just(text_field::InputType::Email),
        Just(text_field::InputType::Url),
        Just(text_field::InputType::Tel),
        Just(text_field::InputType::Search),
    ]
}

fn arb_text_field_props() -> impl Strategy<Value = text_field::Props> {
    (
        prop::option::of(arb_short_text()),
        arb_short_text(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        arb_text_field_input_type(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(
                value,
                default_value,
                disabled,
                readonly,
                invalid,
                required,
                input_type,
                clearable,
                input_mode,
            )| text_field::Props {
                id: "text-field".to_string(),
                value,
                default_value,
                disabled,
                readonly,
                invalid,
                required,
                placeholder: Some("placeholder".to_string()),
                input_type,
                max_length: Some(64),
                min_length: Some(1),
                pattern: Some("[a-z]+".to_string()),
                autocomplete: Some("name".to_string()),
                name: Some("name".to_string()),
                form: Some("form".to_string()),
                clearable,
                dir: Direction::Ltr,
                input_mode: input_mode.then_some(InputMode::Text),
                on_focus_change: None,
                on_value_change: None,
            },
        )
}

fn arb_text_field_event() -> impl Strategy<Value = text_field::Event> {
    prop_oneof![
        any::<bool>().prop_map(|is_keyboard| text_field::Event::Focus { is_keyboard }),
        Just(text_field::Event::Blur),
        arb_short_text().prop_map(text_field::Event::Change),
        Just(text_field::Event::Clear),
        any::<bool>().prop_map(text_field::Event::SetInvalid),
        Just(text_field::Event::CompositionStart),
        arb_short_text().prop_map(text_field::Event::CompositionEnd),
        prop::option::of(arb_short_text()).prop_map(text_field::Event::SetValue),
        Just(text_field::Event::SetProps),
        any::<bool>().prop_map(text_field::Event::SetHasDescription),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_text_field_event_sequences_preserve_invariants(
        props in arb_text_field_props(),
        events in prop::collection::vec(arb_text_field_event(), 0..128),
    ) {
        let mut service = Service::<text_field::Machine>::new(
            props,
            &Env::default(),
            &text_field::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            let ctx = service.context();

            prop_assert_eq!(ctx.ids.id(), "text-field");
            prop_assert_eq!(ctx.focused, service.state() == &text_field::State::Focused);

            if !ctx.focused {
                prop_assert!(!ctx.focus_visible);
            }

            let attrs = service.connect(&|_| {}).input_attrs();

            prop_assert_eq!(attrs.get(&HtmlAttr::Value), Some(ctx.value.get().as_str()));
            prop_assert_eq!(attrs.contains(&HtmlAttr::Disabled), ctx.disabled);
            prop_assert_eq!(attrs.contains(&HtmlAttr::ReadOnly), ctx.readonly);
            prop_assert_eq!(attrs.contains(&HtmlAttr::Required), ctx.required);

            if ctx.invalid {
                prop_assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
            } else {
                prop_assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), None);
            }

            let described_by = match (ctx.has_description, ctx.invalid) {
                (false, false) => None,
                (true, false) => Some("text-field-description"),
                (false, true) => Some("text-field-error-message"),
                (true, true) => Some("text-field-description text-field-error-message"),
            };

            prop_assert_eq!(
                attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
                described_by,
            );
        }
    }
}
