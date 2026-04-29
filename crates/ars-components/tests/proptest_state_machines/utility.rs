use std::collections::BTreeMap;

use ars_components::utility::{
    button, field, fieldset, form, form_submit, separator, visually_hidden,
};
use ars_core::{ConnectApi, Env, HtmlAttr, Service, WeakSend, callback};
use ars_forms::{
    form::Mode,
    validation::{BoxedAsyncValidator, Error},
};
use ars_i18n::Orientation;
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

fn arb_button_props() -> impl Strategy<Value = button::Props> {
    (any::<bool>(), any::<bool>()).prop_map(|(disabled, loading)| {
        button::Props::new()
            .id("button")
            .disabled(disabled)
            .loading(loading)
    })
}

fn arb_button_event() -> impl Strategy<Value = button::Event> {
    prop_oneof![
        any::<bool>().prop_map(|is_keyboard| button::Event::Focus { is_keyboard }),
        Just(button::Event::Blur),
        Just(button::Event::Press),
        Just(button::Event::Release),
        Just(button::Event::Click),
        any::<bool>().prop_map(button::Event::SetLoading),
        any::<bool>().prop_map(button::Event::SetDisabled),
    ]
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
    fn proptest_button_event_sequences_preserve_invariants(
        props in arb_button_props(),
        events in prop::collection::vec(arb_button_event(), 0..128),
    ) {
        let mut service = Service::<button::Machine>::new(
            props,
            &Env::default(),
            &button::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            let state = service.state();
            let ctx = service.context();

            prop_assert_eq!(ctx.loading, matches!(state, button::State::Loading));
            prop_assert_eq!(ctx.pressed, matches!(state, button::State::Pressed));
            prop_assert!(
                !ctx.focus_visible || ctx.focused,
                "focus-visible cannot outlive focus"
            );

            if ctx.disabled {
                prop_assert!(!ctx.focused, "disabled button cannot stay focused");
                prop_assert!(
                    !ctx.focus_visible,
                    "disabled button cannot show focus-visible"
                );
                prop_assert!(!ctx.pressed, "disabled button cannot stay pressed");
            }

            if ctx.loading {
                prop_assert!(!ctx.pressed, "loading button cannot stay pressed");
            }

            match state {
                button::State::Idle => {
                    prop_assert!(!ctx.focused, "idle button cannot stay focused");
                    prop_assert!(
                        !ctx.focus_visible,
                        "idle button cannot show focus-visible"
                    );
                    prop_assert!(!ctx.pressed, "idle button cannot stay pressed");
                }

                button::State::Focused => {
                    prop_assert!(ctx.focused, "focused state requires focused context");
                    prop_assert!(!ctx.pressed, "focused button cannot stay pressed");
                }

                button::State::Pressed => {
                    prop_assert!(ctx.pressed, "pressed state requires pressed context");
                    prop_assert!(!ctx.loading, "pressed button cannot be loading");
                }

                button::State::Loading => {
                    prop_assert!(ctx.loading, "loading state requires loading context");
                    prop_assert!(!ctx.pressed, "loading button cannot be pressed");
                }
            }
        }
    }

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

// ─────────────────────────────────────────────────────────────────────
// Stateless utility components — invariant tests
//
// VisuallyHidden and Separator have no state machine; their `Api` is a
// pure function of `Props` and the only output surface is the `AttrMap`
// returned by `Api::root_attrs()` / `Api::part_attrs(Part::Root)`.
// Proptests here pin the invariants that distinguish output-affecting
// branches and guard against future refactors silently conflating them.
// ─────────────────────────────────────────────────────────────────────

fn arb_short_id() -> impl Strategy<Value = String> {
    // Permit empty, ASCII alnum/punctuation, and unicode-ish ids.
    prop_oneof![
        Just(String::new()),
        "[a-zA-Z0-9_-]{1,32}".prop_map(String::from),
        "[a-zA-Z0-9_-]{0,8} [a-zA-Z0-9_-]{0,8}".prop_map(String::from),
    ]
}

fn arb_orientation() -> impl Strategy<Value = Orientation> {
    prop_oneof![Just(Orientation::Horizontal), Just(Orientation::Vertical)]
}

fn arb_visually_hidden_props() -> impl Strategy<Value = visually_hidden::Props> {
    (arb_short_id(), any::<bool>(), any::<bool>()).prop_map(|(id, as_child, is_focusable)| {
        visually_hidden::Props {
            id,
            as_child,
            is_focusable,
        }
    })
}

fn arb_separator_props() -> impl Strategy<Value = separator::Props> {
    (arb_short_id(), arb_orientation(), any::<bool>()).prop_map(|(id, orientation, decorative)| {
        separator::Props {
            id,
            orientation,
            decorative,
        }
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000)
    ))]

    /// `Api::part_attrs(Part::Root)` always equals `Api::root_attrs()` for
    /// any valid `Props`. Pins the `ConnectApi` dispatch shape.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_visually_hidden_part_root_dispatch_equals_root_attrs(
        props in arb_visually_hidden_props(),
    ) {
        let api = visually_hidden::Api::new(props);

        prop_assert_eq!(api.part_attrs(visually_hidden::Part::Root), api.root_attrs());
    }

    /// `root_attrs()` always carries the canonical scope and part data
    /// attrs. Pins the agnostic-core anatomy contract from spec §2.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_visually_hidden_root_attrs_always_have_scope_and_part(
        props in arb_visually_hidden_props(),
    ) {
        let attrs = visually_hidden::Api::new(props).root_attrs();

        prop_assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("visually-hidden")
        );
        prop_assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
    }

    /// The `is_focusable` flag and the `ars-visually-hidden` class are
    /// mutually exclusive (spec §4 forbids combining them — the class
    /// would clip unconditionally and break focus reveal).
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_visually_hidden_focusable_and_class_are_mutually_exclusive(
        props in arb_visually_hidden_props(),
    ) {
        let is_focusable = props.is_focusable;

        let attrs = visually_hidden::Api::new(props).root_attrs();

        let has_class = attrs.get(&HtmlAttr::Class) == Some("ars-visually-hidden");

        let has_focusable_hook = attrs.contains(&HtmlAttr::Data("ars-visually-hidden-focusable"));

        prop_assert!(
            !(has_class && has_focusable_hook),
            "class and focusable hook must never coexist"
        );
        prop_assert_eq!(has_focusable_hook, is_focusable);
        prop_assert_eq!(has_class, !is_focusable);
    }

    /// `as_child` is an adapter render-path flag and must NOT influence
    /// agnostic-core attribute output.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_visually_hidden_as_child_does_not_affect_root_attrs(
        id in arb_short_id(),
        is_focusable in any::<bool>(),
    ) {
        let without = visually_hidden::Api::new(visually_hidden::Props {
            id: id.clone(),
            as_child: false,
            is_focusable,
        })
        .root_attrs();

        let with = visually_hidden::Api::new(visually_hidden::Props {
            id,
            as_child: true,
            is_focusable,
        })
        .root_attrs();

        prop_assert_eq!(without, with);
    }

    /// `Api::props()` returns a reference to the originally-supplied Props
    /// (round-trip). Pins the F13 escape-hatch contract.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_visually_hidden_api_props_round_trip(
        props in arb_visually_hidden_props(),
    ) {
        let original = props.clone();

        let api = visually_hidden::Api::new(props);

        prop_assert_eq!(api.props(), &original);
        prop_assert_eq!(api.id(), original.id.as_str());
        prop_assert_eq!(api.as_child(), original.as_child);
        prop_assert_eq!(api.is_focusable(), original.is_focusable);
    }

    /// Same dispatch invariant for Separator.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_separator_part_root_dispatch_equals_root_attrs(
        props in arb_separator_props(),
    ) {
        let api = separator::Api::new(props);

        prop_assert_eq!(api.part_attrs(separator::Part::Root), api.root_attrs());
    }

    /// Same scope/part contract for Separator.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_separator_root_attrs_always_have_scope_and_part(
        props in arb_separator_props(),
    ) {
        let attrs = separator::Api::new(props).root_attrs();

        prop_assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("separator"));
        prop_assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
    }

    /// Decorative separators carry `role="none"` and omit both
    /// `aria-orientation` and the `data-ars-orientation` styling hook.
    /// Semantic separators carry `role="separator"` plus `aria-orientation`
    /// and `data-ars-orientation` matching the layout axis. Pins F3/F9.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_separator_decorative_branch_invariants(
        props in arb_separator_props(),
    ) {
        let decorative = props.decorative;

        let orientation = props.orientation;

        let attrs = separator::Api::new(props).root_attrs();

        if decorative {
            prop_assert_eq!(attrs.get(&HtmlAttr::Role), Some("none"));
            prop_assert_eq!(
                attrs.get(&HtmlAttr::Aria(ars_core::AriaAttr::Orientation)),
                None
            );
            prop_assert_eq!(attrs.get(&HtmlAttr::Data("ars-orientation")), None);
            prop_assert_eq!(
                attrs.get(&HtmlAttr::Aria(ars_core::AriaAttr::Hidden)),
                None
            );
        } else {
            let expected = match orientation {
                Orientation::Horizontal => "horizontal",
                Orientation::Vertical => "vertical",
            };

            prop_assert_eq!(attrs.get(&HtmlAttr::Role), Some("separator"));
            prop_assert_eq!(
                attrs.get(&HtmlAttr::Aria(ars_core::AriaAttr::Orientation)),
                Some(expected)
            );
            prop_assert_eq!(
                attrs.get(&HtmlAttr::Data("ars-orientation")),
                Some(expected)
            );
        }
    }

    /// For decorative separators, `orientation` is invisible to the
    /// agnostic-core output. Pins the "decorative collapses orientation"
    /// invariant tested under one example in the unit suite.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_separator_decorative_orientation_does_not_affect_output(
        id in arb_short_id(),
    ) {
        let h = separator::Api::new(separator::Props {
            id: id.clone(),
            orientation: Orientation::Horizontal,
            decorative: true,
        })
        .root_attrs();

        let v = separator::Api::new(separator::Props {
            id,
            orientation: Orientation::Vertical,
            decorative: true,
        })
        .root_attrs();

        prop_assert_eq!(h, v);
    }

    /// `Api::props()` round-trip for Separator.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_separator_api_props_round_trip(
        props in arb_separator_props(),
    ) {
        let original = props.clone();

        let api = separator::Api::new(props);

        prop_assert_eq!(api.props(), &original);
        prop_assert_eq!(api.id(), original.id.as_str());
        prop_assert_eq!(api.orientation(), original.orientation);
        prop_assert_eq!(api.decorative(), original.decorative);
    }
}
