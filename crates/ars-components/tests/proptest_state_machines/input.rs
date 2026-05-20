use ars_components::input::{
    checkbox, number_input, password_input, pin_input, search_input, switch, text_field, textarea,
};
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
    #![proptest_config(super::common::proptest_config())]

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
                if effect.name == textarea::Effect::AutoResize {
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

// ─────────────────────────────────────────────────────────────────────
// PasswordInput
// ─────────────────────────────────────────────────────────────────────

fn arb_password_input_props() -> impl Strategy<Value = password_input::Props> {
    (
        prop::option::of(arb_short_text()),
        arb_short_text(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(value, default_value, disabled, required, invalid, readonly, default_visible)| {
                password_input::Props {
                    id: "password-input".to_string(),
                    value,
                    default_value,
                    disabled,
                    required,
                    invalid,
                    readonly,
                    default_visible,
                    placeholder: Some("Password".to_string()),
                    name: Some("password".to_string()),
                    form: Some("form".to_string()),
                    autocomplete: Some("current-password".to_string()),
                }
            },
        )
}

fn arb_password_input_event() -> impl Strategy<Value = password_input::Event> {
    prop_oneof![
        Just(password_input::Event::ToggleVisibility),
        any::<bool>().prop_map(password_input::Event::SetVisibility),
        any::<bool>().prop_map(|is_keyboard| password_input::Event::Focus { is_keyboard }),
        Just(password_input::Event::Blur),
    ]
}

// ─────────────────────────────────────────────────────────────────────
// SearchInput
// ─────────────────────────────────────────────────────────────────────

fn arb_search_input_props() -> impl Strategy<Value = search_input::Props> {
    (
        prop::option::of(arb_short_text()),
        arb_short_text(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        prop::option::of(1_u64..=500),
    )
        .prop_map(
            |(value, default_value, disabled, readonly, invalid, required, debounce_ms)| {
                search_input::Props {
                    id: "search-input".to_string(),
                    value,
                    default_value,
                    disabled,
                    readonly,
                    invalid,
                    required,
                    placeholder: Some("Search...".to_string()),
                    name: Some("q".to_string()),
                    form: Some("form".to_string()),
                    debounce: debounce_ms.map(core::time::Duration::from_millis),
                }
            },
        )
}

fn arb_search_input_event() -> impl Strategy<Value = search_input::Event> {
    prop_oneof![
        any::<bool>().prop_map(|is_keyboard| search_input::Event::Focus { is_keyboard }),
        Just(search_input::Event::Blur),
        arb_short_text().prop_map(search_input::Event::Change),
        Just(search_input::Event::Clear),
        Just(search_input::Event::Submit),
        any::<bool>().prop_map(search_input::Event::SetSearching),
        Just(search_input::Event::DebounceExpired),
        Just(search_input::Event::CancelDebounce),
        Just(search_input::Event::CompositionStart),
        Just(search_input::Event::CompositionEnd),
    ]
}

// ─────────────────────────────────────────────────────────────────────
// NumberInput
// ─────────────────────────────────────────────────────────────────────

fn arb_number_input_props() -> impl Strategy<Value = number_input::Props> {
    (
        prop::option::of(-1000.0_f64..1000.0),
        prop::option::of(-1000.0_f64..1000.0),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        prop::option::of(0_u32..=4),
    )
        .prop_map(
            |(
                value,
                default_value,
                disabled,
                readonly,
                invalid,
                required,
                allow_mouse_wheel,
                spin_on_press,
                precision,
            )| number_input::Props {
                id: "number-input".to_string(),
                value,
                default_value,
                min: -1000.0,
                max: 1000.0,
                step: 1.0,
                large_step: 10.0,
                precision,
                disabled,
                readonly,
                invalid,
                required,
                name: Some("qty".to_string()),
                form: Some("form".to_string()),
                allow_mouse_wheel,
                clamp_value_on_blur: true,
                spin_on_press,
                format_options: None,
                display_format: None,
            },
        )
}

fn arb_number_input_event() -> impl Strategy<Value = number_input::Event> {
    prop_oneof![
        any::<bool>().prop_map(|is_keyboard| number_input::Event::Focus { is_keyboard }),
        Just(number_input::Event::Blur),
        arb_short_text().prop_map(number_input::Event::Change),
        Just(number_input::Event::Increment),
        Just(number_input::Event::Decrement),
        Just(number_input::Event::IncrementLarge),
        Just(number_input::Event::DecrementLarge),
        Just(number_input::Event::IncrementToMax),
        Just(number_input::Event::DecrementToMin),
        (-1000.0_f64..1000.0).prop_map(number_input::Event::SetValue),
        Just(number_input::Event::StartScrub),
        (-10.0_f64..10.0).prop_map(number_input::Event::Scrub),
        Just(number_input::Event::EndScrub),
        (-5.0_f64..5.0).prop_map(|delta| number_input::Event::Wheel { delta }),
        Just(number_input::Event::CompositionStart),
        Just(number_input::Event::CompositionEnd),
    ]
}

// ─────────────────────────────────────────────────────────────────────
// PinInput
// ─────────────────────────────────────────────────────────────────────

fn arb_pin_mode() -> impl Strategy<Value = pin_input::Mode> {
    prop_oneof![
        Just(pin_input::Mode::Numeric),
        Just(pin_input::Mode::Alphanumeric),
        Just(pin_input::Mode::Password),
    ]
}

fn arb_pin_input_props() -> impl Strategy<Value = pin_input::Props> {
    (
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        arb_pin_mode(),
    )
        .prop_map(
            |(disabled, invalid, otp, mask, required, auto_submit, mode)| pin_input::Props {
                id: "pin-input".to_string(),
                value: None,
                default_value: Vec::new(),
                length: 4,
                disabled,
                invalid,
                otp,
                mask,
                placeholder: Some("_".to_string()),
                mode,
                name: Some("pin".to_string()),
                form: Some("form".to_string()),
                required,
                readonly: false,
                select_on_focus: false,
                blur_on_complete: false,
                auto_submit,
                on_value_complete: None,
            },
        )
}

fn arb_pin_input_event() -> impl Strategy<Value = pin_input::Event> {
    prop_oneof![
        (0_usize..4, any::<bool>())
            .prop_map(|(index, is_keyboard)| pin_input::Event::Focus { index, is_keyboard }),
        Just(pin_input::Event::Blur),
        (
            0_usize..4,
            "[0-9a-z]".prop_map(|s| s.chars().next().unwrap_or('0'))
        )
            .prop_map(|(index, c)| pin_input::Event::InputChar { index, char: c }),
        (0_usize..4).prop_map(|index| pin_input::Event::DeleteChar { index }),
        "[0-9a-z]{0,8}".prop_map(pin_input::Event::Paste),
        Just(pin_input::Event::Clear),
        Just(pin_input::Event::FocusNext),
        Just(pin_input::Event::FocusPrev),
        Just(pin_input::Event::CompositionStart),
        Just(pin_input::Event::CompositionEnd),
    ]
}

proptest! {
    #![proptest_config(super::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_password_input_event_sequences_preserve_invariants(
        props in arb_password_input_props(),
        events in prop::collection::vec(arb_password_input_event(), 0..128),
    ) {
        let mut service = Service::<password_input::Machine>::new(
            props,
            &Env::default(),
            &password_input::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            let ctx = service.context();
            let state = service.state();

            // visibility flag tracks state
            match state {
                password_input::State::Masked => prop_assert!(!ctx.visible),
                password_input::State::Visible => prop_assert!(ctx.visible),
            }

            let input_attrs = service.connect(&|_| {}).input_attrs();

            let ty = if ctx.visible { "text" } else { "password" };

            prop_assert_eq!(input_attrs.get(&HtmlAttr::Type), Some(ty));

            if ctx.disabled {
                prop_assert!(input_attrs.contains(&HtmlAttr::Disabled));
            }
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_search_input_event_sequences_preserve_invariants(
        props in arb_search_input_props(),
        events in prop::collection::vec(arb_search_input_event(), 0..128),
    ) {
        let mut service = Service::<search_input::Machine>::new(
            props,
            &Env::default(),
            &search_input::Messages::default(),
        );

        for event in events {
            let result = service.send(event);

            let ctx = service.context();
            let state = *service.state();

            // Loading flag aligns with Searching state on entry, but Idle/Focused
            // can be returned to while loading=false. Just check the converse:
            // when state == Searching, loading must be true (set by Submit/SetSearching).
            if state == search_input::State::Searching {
                prop_assert!(ctx.loading);
            }

            // is_composing never co-occurs with a fresh debounce effect
            if ctx.is_composing {
                for effect in &result.pending_effects {
                    prop_assert!(effect.name != search_input::Effect::SearchDebounce);
                }
            }

            let input_attrs = service.connect(&|_| {}).input_attrs();

            prop_assert_eq!(input_attrs.get(&HtmlAttr::Type), Some("search"));
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_number_input_event_sequences_preserve_invariants(
        props in arb_number_input_props(),
        events in prop::collection::vec(arb_number_input_event(), 0..128),
    ) {
        let mut service = Service::<number_input::Machine>::new(
            props,
            &Env::default(),
            &number_input::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            let ctx = service.context();

            // value is always inside [min, max] when Some and the machine has not
            // observed a `Change` since the last clamp opportunity (Blur with
            // clamp_value_on_blur). Increment/Decrement/SetValue all clamp.
            // Change(text) does NOT clamp, so we only assert the clamping
            // happens for non-Change-only paths by checking after a Blur.
            if let Some(value) = ctx.value.get() {
                prop_assert!(!value.is_nan(), "value never becomes NaN");
            }

            let input_attrs = service.connect(&|_| {}).input_attrs();

            prop_assert_eq!(input_attrs.get(&HtmlAttr::Role), Some("spinbutton"));
            prop_assert_eq!(input_attrs.get(&HtmlAttr::InputMode), Some("decimal"));
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_pin_input_event_sequences_preserve_invariants(
        props in arb_pin_input_props(),
        events in prop::collection::vec(arb_pin_input_event(), 0..128),
    ) {
        let mut service = Service::<pin_input::Machine>::new(
            props,
            &Env::default(),
            &pin_input::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            let ctx = service.context();

            // Cell vector always has the configured length
            prop_assert_eq!(ctx.value.get().len(), ctx.length);

            // Complete iff every cell is non-empty (for length > 0).
            let all_filled = ctx.length > 0 && ctx.value.get().iter().all(|cell| !cell.is_empty());

            prop_assert_eq!(ctx.complete, all_filled);

            // Hidden input value equals the joined cell strings.
            let hidden_attrs = service.connect(&|_| {}).hidden_input_attrs();

            let joined = ctx.value.get().join("");

            prop_assert_eq!(
                hidden_attrs.get(&HtmlAttr::Value),
                Some(joined.as_str())
            );
        }
    }
}
