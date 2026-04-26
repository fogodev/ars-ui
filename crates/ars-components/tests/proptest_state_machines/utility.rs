use std::collections::BTreeMap;

use ars_components::utility::{field, fieldset, form, form_submit};
use ars_core::{Env, Service, WeakSend, callback};
use ars_forms::{
    form::Mode,
    validation::{BoxedAsyncValidator, Error},
};
use proptest::prelude::*;

fn arb_direction() -> impl Strategy<Value = Option<ars_core::Direction>> {
    prop_oneof![
        Just(None),
        Just(Some(ars_core::Direction::Ltr)),
        Just(Some(ars_core::Direction::Rtl)),
    ]
}

fn arb_error() -> impl Strategy<Value = Error> {
    (
        "[a-z]{1,8}".prop_map(String::from),
        "[a-zA-Z0-9 _-]{1,16}".prop_map(String::from),
    )
        .prop_map(|(code, message)| Error::custom(code, message))
}

fn arb_field_props() -> impl Strategy<Value = field::Props> {
    (
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        arb_direction(),
    )
        .prop_map(
            |(required, disabled, readonly, invalid, dir)| field::Props {
                id: "field".to_string(),
                required,
                disabled,
                readonly,
                invalid,
                dir,
            },
        )
}

fn arb_field_event() -> impl Strategy<Value = field::Event> {
    prop_oneof![
        prop::collection::vec(arb_error(), 0..4).prop_map(field::Event::SetErrors),
        Just(field::Event::ClearErrors),
        any::<bool>().prop_map(field::Event::SetHasDescription),
        any::<bool>().prop_map(field::Event::SetDisabled),
        any::<bool>().prop_map(field::Event::SetInvalid),
        any::<bool>().prop_map(field::Event::SetReadonly),
        any::<bool>().prop_map(field::Event::SetRequired),
        arb_direction().prop_map(field::Event::SetDir),
        any::<bool>().prop_map(field::Event::SetValidating),
    ]
}

fn arb_fieldset_props() -> impl Strategy<Value = fieldset::Props> {
    (any::<bool>(), any::<bool>(), any::<bool>(), arb_direction()).prop_map(
        |(disabled, invalid, readonly, dir)| fieldset::Props {
            id: "fieldset".to_string(),
            disabled,
            invalid,
            readonly,
            dir,
        },
    )
}

fn arb_fieldset_event() -> impl Strategy<Value = fieldset::Event> {
    prop_oneof![
        prop::collection::vec(arb_error(), 0..4).prop_map(fieldset::Event::SetErrors),
        Just(fieldset::Event::ClearErrors),
        any::<bool>().prop_map(fieldset::Event::SetDisabled),
        any::<bool>().prop_map(fieldset::Event::SetInvalid),
        any::<bool>().prop_map(fieldset::Event::SetReadonly),
        arb_direction().prop_map(fieldset::Event::SetDir),
        any::<bool>().prop_map(fieldset::Event::SetHasDescription),
    ]
}

fn arb_validation_behavior() -> impl Strategy<Value = form::ValidationBehavior> {
    prop_oneof![
        Just(form::ValidationBehavior::Aria),
        Just(form::ValidationBehavior::Native),
    ]
}

fn arb_error_map() -> impl Strategy<Value = BTreeMap<String, Vec<String>>> {
    prop::collection::btree_map(
        "[a-z]{1,8}".prop_map(String::from),
        prop::collection::vec("[a-zA-Z0-9 _-]{1,16}".prop_map(String::from), 1..4),
        0..4,
    )
}

fn arb_form_props() -> impl Strategy<Value = form::Props> {
    (
        arb_validation_behavior(),
        arb_error_map(),
        prop::option::of("[a-zA-Z0-9:/._?#=-]{1,24}".prop_map(String::from)),
        prop::option::of("[a-z-]{1,12}".prop_map(String::from)),
    )
        .prop_map(
            |(validation_behavior, validation_errors, action, role)| form::Props {
                id: "form".to_string(),
                validation_behavior,
                validation_errors,
                action,
                role,
            },
        )
}

fn arb_form_event() -> impl Strategy<Value = form::Event> {
    prop_oneof![
        Just(form::Event::Submit),
        any::<bool>().prop_map(|success| form::Event::SubmitComplete { success }),
        Just(form::Event::Reset),
        arb_error_map().prop_map(form::Event::SetServerErrors),
        Just(form::Event::ClearServerErrors),
        arb_validation_behavior().prop_map(form::Event::SetValidationBehavior),
        prop::option::of("[a-zA-Z0-9 _-]{1,16}".prop_map(String::from))
            .prop_map(form::Event::SetStatusMessage),
    ]
}

fn arb_mode() -> impl Strategy<Value = Mode> {
    prop_oneof![
        Just(Mode::on_submit()),
        Just(Mode::on_blur_revalidate()),
        Just(Mode::on_change()),
    ]
}

fn arb_server_errors() -> impl Strategy<Value = BTreeMap<String, Vec<String>>> {
    prop::collection::btree_map(
        "[a-z]{1,8}".prop_map(String::from),
        prop::collection::vec("[a-zA-Z0-9 _-]{1,16}".prop_map(String::from), 1..4),
        0..4,
    )
}

fn arb_form_submit_event() -> impl Strategy<Value = form_submit::Event> {
    prop_oneof![
        Just(form_submit::Event::Submit),
        Just(form_submit::Event::ValidationPassed),
        Just(form_submit::Event::ValidationFailed),
        Just(form_submit::Event::SubmitComplete),
        "[a-zA-Z0-9 _-]{1,16}"
            .prop_map(String::from)
            .prop_map(form_submit::Event::SubmitError),
        Just(form_submit::Event::Reset),
        arb_server_errors().prop_map(form_submit::Event::SetServerErrors),
        arb_mode().prop_map(form_submit::Event::SetMode),
    ]
}

fn form_submit_props(initial_mode: Mode) -> form_submit::Props {
    form_submit::Props {
        id: "test-form".into(),
        validation_mode: initial_mode,
        spawn_async_validation: callback(
            |_: (
                Vec<(String, BoxedAsyncValidator)>,
                WeakSend<form_submit::Event>,
            )|
             -> Box<dyn FnOnce()> { Box::new(|| {}) },
        ),
        schedule_microtask: callback(|_: Box<dyn FnOnce()>| {}),
    }
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
    fn proptest_field_event_sequences_preserve_invariants(
        props in arb_field_props(),
        events in prop::collection::vec(arb_field_event(), 0..128),
    ) {
        let mut service = Service::<field::Machine>::new(props, &Env::default(), &());

        for event in events {
            drop(service.send(event));

            prop_assert_eq!(service.state(), &field::State::Idle);
            prop_assert_eq!(service.context().ids.id(), "field");
            prop_assert!(service.context().errors.is_empty() || service.context().invalid);
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_fieldset_event_sequences_preserve_invariants(
        props in arb_fieldset_props(),
        events in prop::collection::vec(arb_fieldset_event(), 0..128),
    ) {
        let mut service = Service::<fieldset::Machine>::new(props, &Env::default(), &());

        for event in events {
            drop(service.send(event));

            prop_assert_eq!(service.state(), &fieldset::State::Idle);
            prop_assert_eq!(service.context().ids.id(), "fieldset");
            prop_assert!(service.context().errors.is_empty() || service.context().invalid);
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_form_event_sequences_preserve_invariants(
        props in arb_form_props(),
        events in prop::collection::vec(arb_form_event(), 0..128),
    ) {
        let mut service = Service::<form::Machine>::new(props, &Env::default(), &());

        for event in events {
            drop(service.send(event));

            prop_assert_eq!(
                service.context().is_submitting,
                matches!(service.state(), form::State::Submitting)
            );
            prop_assert_eq!(service.context().ids.id(), "form");
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_form_submit_event_sequences_preserve_invariants(
        initial_mode in arb_mode(),
        events in prop::collection::vec(arb_form_submit_event(), 0..128),
    ) {
        let mut service = Service::<form_submit::Machine>::new(
            form_submit_props(initial_mode),
            &Env::default(),
            &(),
        );

        for event in events {
            drop(service.send(event));

            prop_assert_eq!(
                service.context().form.is_submitting,
                matches!(service.state(), form_submit::State::Submitting)
            );
            prop_assert_eq!(
                service.context().submit_error.is_some(),
                matches!(service.state(), form_submit::State::Failed)
            );
            prop_assert_eq!(service.context().ids.id(), "test-form");
        }
    }
}
