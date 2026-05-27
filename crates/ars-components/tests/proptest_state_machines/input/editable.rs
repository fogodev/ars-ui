use ars_components::input::editable;
use ars_core::{Env, HtmlAttr, Service};
use proptest::prelude::*;

use super::arb_short_text;

#[derive(Clone, Debug)]
enum EditableAction {
    Activate,
    Change(String),
    Submit(String),
    Cancel,
    Focus(bool),
    Blur,
    CompositionStart,
    CompositionEnd(String),
}

fn arb_editable_props() -> impl Strategy<Value = editable::Props> {
    (
        arb_short_text(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(default_value, disabled, readonly, invalid, required)| editable::Props {
                id: "editable".to_string(),
                value: None,
                default_value,
                disabled,
                readonly,
                submit_mode: editable::SubmitMode::Both,
                activate_mode: editable::ActivateMode::DblClick,
                auto_select: true,
                placeholder: Some("placeholder".to_string()),
                max_length: Some(8),
                invalid,
                required,
                name: Some("name".to_string()),
                form: Some("form".to_string()),
                submit_on_blur: true,
            },
        )
}

fn arb_editable_action() -> impl Strategy<Value = EditableAction> {
    prop_oneof![
        Just(EditableAction::Activate),
        arb_short_text().prop_map(EditableAction::Change),
        arb_short_text().prop_map(EditableAction::Submit),
        Just(EditableAction::Cancel),
        any::<bool>().prop_map(EditableAction::Focus),
        Just(EditableAction::Blur),
        Just(EditableAction::CompositionStart),
        arb_short_text().prop_map(EditableAction::CompositionEnd),
    ]
}

fn clamp_editable_text(value: &str) -> String {
    value.chars().take(8).collect()
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_editable_event_sequences_preserve_invariants(
        props in arb_editable_props(),
        actions in prop::collection::vec(arb_editable_action(), 0..128),
    ) {
        let mut service = Service::<editable::Machine>::new(
            props.clone(),
            &Env::default(),
            &editable::Messages::default(),
        );

        let mut expected_committed = props.default_value.clone();
        let mut expected_edit = props.default_value.clone();
        let mut expected_state = editable::State::Preview;
        let mut expected_composing = false;

        for action in actions {
            let event = match action {
                EditableAction::Activate => {
                    if !props.disabled && !props.readonly && expected_state == editable::State::Preview {
                        expected_state = editable::State::Editing;
                        expected_edit = expected_committed.clone();
                    }

                    editable::Event::Activate
                }

                EditableAction::Change(value) => {
                    if expected_state == editable::State::Editing
                        && !props.disabled
                        && !props.readonly
                        && !expected_composing
                    {
                        expected_edit = clamp_editable_text(&value);
                    }

                    editable::Event::Change(value)
                }

                EditableAction::Submit(value) => {
                    if expected_state == editable::State::Editing
                        && !props.disabled
                        && !props.readonly
                        && !expected_composing
                    {
                        let value = clamp_editable_text(&value);

                        expected_committed = value.clone();
                        expected_edit = value;
                        expected_state = editable::State::Preview;
                        expected_composing = false;
                    }

                    editable::Event::Submit(value)
                }

                EditableAction::Cancel => {
                    if expected_state == editable::State::Editing {
                        expected_edit = expected_committed.clone();
                        expected_state = editable::State::Preview;
                        expected_composing = false;
                    }

                    editable::Event::Cancel
                }

                EditableAction::Focus(is_keyboard) => editable::Event::Focus { is_keyboard },

                EditableAction::Blur => {
                    if expected_state == editable::State::Editing && !expected_composing {
                        expected_committed = expected_edit.clone();
                        expected_state = editable::State::Preview;
                        expected_composing = false;
                    }

                    editable::Event::Blur
                }

                EditableAction::CompositionStart => {
                    expected_composing = true;

                    editable::Event::CompositionStart
                }

                EditableAction::CompositionEnd(value) => {
                    expected_composing = false;

                    if expected_state == editable::State::Editing && !props.disabled && !props.readonly {
                        expected_edit = clamp_editable_text(&value);
                    }

                    editable::Event::CompositionEnd(value)
                }
            };

            drop(service.send(event));

            let ctx = service.context();

            let attrs = service.connect(&|_| {}).input_attrs();

            prop_assert_eq!(ctx.ids.id(), "editable");
            prop_assert_eq!(*service.state(), expected_state);
            prop_assert_eq!(ctx.value.get(), &expected_committed);
            prop_assert_eq!(&ctx.edit_value, &expected_edit);
            prop_assert_eq!(ctx.is_composing, expected_composing);
            prop_assert_eq!(attrs.get(&HtmlAttr::Value), Some(ctx.edit_value.as_str()));
            prop_assert_eq!(attrs.contains(&HtmlAttr::Disabled), ctx.disabled);
            prop_assert_eq!(attrs.contains(&HtmlAttr::ReadOnly), ctx.readonly);

            if ctx.disabled || ctx.readonly {
                prop_assert_ne!(*service.state(), editable::State::Editing);
            }
        }
    }
}
