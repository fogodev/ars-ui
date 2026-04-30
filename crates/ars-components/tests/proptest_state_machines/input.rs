use ars_components::input::{checkbox, switch, text_field, textarea};
use ars_core::{AriaAttr, Direction, EffectMetadata, Env, HtmlAttr, InputMode, Service};
use proptest::prelude::*;

fn arb_checkbox_state() -> impl Strategy<Value = checkbox::State> {
    prop_oneof![
        Just(checkbox::State::Unchecked),
        Just(checkbox::State::Checked),
        Just(checkbox::State::Indeterminate),
    ]
}

fn arb_checkbox_props() -> impl Strategy<Value = checkbox::Props> {
    (
        prop::option::of(arb_checkbox_state()),
        arb_checkbox_state(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        prop::option::of("[a-z]{1,8}".prop_map(String::from)),
        prop::option::of("[a-z]{1,8}".prop_map(String::from)),
        "[a-z]{1,8}".prop_map(String::from),
    )
        .prop_map(
            |(
                checked,
                default_checked,
                disabled,
                required,
                invalid,
                readonly,
                name,
                form,
                value,
            )| {
                checkbox::Props {
                    id: "checkbox".to_string(),
                    checked,
                    default_checked,
                    disabled,
                    required,
                    invalid,
                    readonly,
                    name,
                    form,
                    value,
                    on_checked_change: None,
                }
            },
        )
}

fn arb_checkbox_event() -> impl Strategy<Value = checkbox::Event> {
    prop_oneof![
        Just(checkbox::Event::Toggle),
        Just(checkbox::Event::Check),
        Just(checkbox::Event::Uncheck),
        Just(checkbox::Event::Reset),
        prop::option::of(arb_checkbox_state()).prop_map(checkbox::Event::SetValue),
        Just(checkbox::Event::SetProps),
        any::<bool>().prop_map(checkbox::Event::SetHasDescription),
        any::<bool>().prop_map(|is_keyboard| checkbox::Event::Focus { is_keyboard }),
        Just(checkbox::Event::Blur),
    ]
}

const fn aria_checked_token(state: checkbox::State) -> &'static str {
    match state {
        checkbox::State::Unchecked => "false",
        checkbox::State::Checked => "true",
        checkbox::State::Indeterminate => "mixed",
    }
}

fn arb_switch_props() -> impl Strategy<Value = switch::Props> {
    (
        prop::option::of(any::<bool>()),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        prop::option::of("[a-z]{1,8}".prop_map(String::from)),
        prop::option::of("[a-z]{1,8}".prop_map(String::from)),
        "[a-z]{1,8}".prop_map(String::from),
        prop::option::of("[a-z]{1,8}".prop_map(String::from)),
        any::<bool>(),
    )
        .prop_map(
            |(
                checked,
                default_checked,
                disabled,
                required,
                invalid,
                readonly,
                name,
                form,
                value,
                label,
                rtl,
            )| switch::Props {
                id: "switch".to_string(),
                checked,
                default_checked,
                disabled,
                required,
                invalid,
                readonly,
                name,
                form,
                value,
                label,
                dir: if rtl { Direction::Rtl } else { Direction::Ltr },
                on_checked_change: None,
            },
        )
}

fn arb_switch_event() -> impl Strategy<Value = switch::Event> {
    prop_oneof![
        Just(switch::Event::Toggle),
        Just(switch::Event::TurnOn),
        Just(switch::Event::TurnOff),
        Just(switch::Event::Reset),
        prop::option::of(any::<bool>()).prop_map(switch::Event::SetValue),
        Just(switch::Event::SetProps),
        any::<bool>().prop_map(switch::Event::SetHasDescription),
        any::<bool>().prop_map(|is_keyboard| switch::Event::Focus { is_keyboard }),
        Just(switch::Event::Blur),
    ]
}

const fn switch_state_checked(state: switch::State) -> bool {
    match state {
        switch::State::Off => false,
        switch::State::On => true,
    }
}

const fn switch_aria_checked_token(state: switch::State) -> &'static str {
    match state {
        switch::State::Off => "false",
        switch::State::On => "true",
    }
}

fn arb_short_text() -> impl Strategy<Value = String> {
    "[a-z]{0,16}".prop_map(String::from)
}

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

fn arb_textarea_resize() -> impl Strategy<Value = textarea::ResizeMode> {
    prop_oneof![
        Just(textarea::ResizeMode::None),
        Just(textarea::ResizeMode::Both),
        Just(textarea::ResizeMode::Horizontal),
        Just(textarea::ResizeMode::Vertical),
    ]
}

fn arb_textarea_props() -> impl Strategy<Value = textarea::Props> {
    (
        prop::option::of(arb_short_text()),
        arb_short_text(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        1_u32..12,
        prop::option::of(1_u32..80),
        arb_textarea_resize(),
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
                rows,
                cols,
                resize,
                auto_resize,
                input_mode,
            )| textarea::Props {
                id: "textarea".to_string(),
                value,
                default_value,
                disabled,
                readonly,
                invalid,
                required,
                placeholder: Some("placeholder".to_string()),
                max_length: Some(256),
                min_length: Some(1),
                name: Some("bio".to_string()),
                form: Some("form".to_string()),
                autocomplete: Some("off".to_string()),
                rows,
                cols,
                resize,
                auto_resize,
                max_height: Some("240px".to_string()),
                max_rows: Some(8),
                dir: Direction::Ltr,
                input_mode: input_mode.then_some(InputMode::Text),
                on_value_change: None,
            },
        )
}

fn arb_textarea_event() -> impl Strategy<Value = textarea::Event> {
    prop_oneof![
        any::<bool>().prop_map(|is_keyboard| textarea::Event::Focus { is_keyboard }),
        Just(textarea::Event::Blur),
        arb_short_text().prop_map(textarea::Event::Change),
        Just(textarea::Event::Clear),
        any::<bool>().prop_map(textarea::Event::SetInvalid),
        Just(textarea::Event::CompositionStart),
        arb_short_text().prop_map(textarea::Event::CompositionEnd),
        prop::option::of(arb_short_text()).prop_map(textarea::Event::SetValue),
        Just(textarea::Event::SetProps),
        any::<bool>().prop_map(textarea::Event::SetHasDescription),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000)
    ))]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_checkbox_event_sequences_preserve_invariants(
        props in arb_checkbox_props(),
        events in prop::collection::vec(arb_checkbox_event(), 0..128),
    ) {
        let mut service = Service::<checkbox::Machine>::new(
            props,
            &Env::default(),
            &checkbox::Messages,
        );

        for event in events {
            drop(service.send(event));

            let state = *service.state();

            prop_assert_eq!(service.context().ids.id(), "checkbox");
            prop_assert_eq!(service.context().checked.get(), &state);

            let control_attrs = service.connect(&|_| {}).control_attrs();

            prop_assert_eq!(
                control_attrs.get(&HtmlAttr::Aria(AriaAttr::Checked)),
                Some(aria_checked_token(state))
            );

            let hidden_input_attrs = service.connect(&|_| {}).hidden_input_attrs();

            if service.context().disabled {
                prop_assert!(hidden_input_attrs.contains(&HtmlAttr::Disabled));
            }

            if service.context().required {
                prop_assert!(hidden_input_attrs.contains(&HtmlAttr::Required));
            }
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_switch_event_sequences_preserve_invariants(
        props in arb_switch_props(),
        events in prop::collection::vec(arb_switch_event(), 0..128),
    ) {
        let mut service = Service::<switch::Machine>::new(
            props,
            &Env::default(),
            &switch::Messages,
        );

        for event in events {
            drop(service.send(event));

            let state = *service.state();

            prop_assert_eq!(service.context().ids.id(), "switch");
            prop_assert_eq!(service.context().checked.get(), &switch_state_checked(state));

            let control_attrs = service.connect(&|_| {}).control_attrs();

            prop_assert_eq!(control_attrs.get(&HtmlAttr::Role), Some("switch"));
            prop_assert_eq!(
                control_attrs.get(&HtmlAttr::Aria(AriaAttr::Checked)),
                Some(switch_aria_checked_token(state))
            );

            let hidden_input_attrs = service.connect(&|_| {}).hidden_input_attrs();

            if service.context().disabled {
                prop_assert!(hidden_input_attrs.contains(&HtmlAttr::Disabled));
            }

            if service.context().required {
                prop_assert!(hidden_input_attrs.contains(&HtmlAttr::Required));
            }

            let described_by = match (service.context().has_description, service.context().invalid) {
                (false, false) => None,
                (true, false) => Some("switch-description"),
                (false, true) => Some("switch-error-message"),
                (true, true) => Some("switch-description switch-error-message"),
            };

            prop_assert_eq!(
                control_attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
                described_by,
            );
        }
    }

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

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_textarea_event_sequences_preserve_invariants(
        props in arb_textarea_props(),
        events in prop::collection::vec(arb_textarea_event(), 0..128),
    ) {
        let mut service = Service::<textarea::Machine>::new(
            props,
            &Env::default(),
            &textarea::Messages,
        );

        for event in events {
            let result = service.send(event);

            let ctx = service.context();

            prop_assert_eq!(ctx.ids.id(), "textarea");
            prop_assert_eq!(ctx.focused, service.state() == &textarea::State::Focused);

            if !ctx.focused {
                prop_assert!(!ctx.focus_visible);
            }

            let attrs = service.connect(&|_| {}).textarea_attrs();

            prop_assert_eq!(attrs.get(&HtmlAttr::Value), Some(ctx.value.get().as_str()));
            prop_assert_eq!(attrs.contains(&HtmlAttr::Disabled), ctx.disabled);
            prop_assert_eq!(attrs.contains(&HtmlAttr::ReadOnly), ctx.readonly);
            prop_assert_eq!(attrs.contains(&HtmlAttr::Required), ctx.required);

            let rows = ctx.rows.to_string();

            prop_assert_eq!(attrs.get(&HtmlAttr::Rows), Some(rows.as_str()));

            if ctx.invalid {
                prop_assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
            } else {
                prop_assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), None);
            }

            let described_by = match (ctx.has_description, ctx.invalid) {
                (false, false) => None,
                (true, false) => Some("textarea-description"),
                (false, true) => Some("textarea-error-message"),
                (true, true) => Some("textarea-description textarea-error-message"),
            };

            prop_assert_eq!(
                attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
                described_by,
            );

            for effect in &result.pending_effects {
                if effect.name == "auto-resize" {
                    prop_assert_eq!(
                        effect.metadata.as_ref(),
                        Some(&EffectMetadata::ResizeToContent(ars_core::ResizeToContentEffect {
                            element_id: "textarea-textarea".to_string(),
                            max_height: ctx.max_height.clone(),
                            max_rows: ctx.max_rows,
                        })),
                    );
                }
            }
        }
    }
}
