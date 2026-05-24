use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use ars_a11y::AriaRelevant;
use ars_collections::{Key, selection};
#[cfg(feature = "i18n")]
use ars_components::utility::highlight;
use ars_components::utility::{
    action_group, button, download_trigger, drop_zone, error_boundary, field, fieldset,
    focus_scope, form, form_submit, live_region, separator, swap, toggle, toggle_group,
    visually_hidden,
};
use ars_core::{ConnectApi, Env, HtmlAttr, Service, WeakSend, callback};
use ars_forms::{
    form::Mode,
    validation::{BoxedAsyncValidator, Error},
};
#[cfg(feature = "i18n")]
use ars_i18n::Locale;
use ars_i18n::Orientation;
use ars_interactions::{DragItem, FileHandle};
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

fn arb_toggle_props() -> impl Strategy<Value = toggle::Props> {
    (
        prop::option::of(any::<bool>()),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(pressed, default_pressed, disabled)| toggle::Props {
            id: "toggle".to_string(),
            pressed,
            default_pressed,
            disabled,
            on_change: None,
        })
}

fn arb_toggle_event() -> impl Strategy<Value = toggle::Event> {
    prop_oneof![
        Just(toggle::Event::Toggle),
        Just(toggle::Event::TurnOn),
        Just(toggle::Event::TurnOff),
        any::<bool>().prop_map(|is_keyboard| toggle::Event::Focus { is_keyboard }),
        Just(toggle::Event::Blur),
        any::<bool>().prop_map(toggle::Event::SetDisabled),
        prop::option::of(any::<bool>()).prop_map(toggle::Event::SetValue),
    ]
}

fn arb_drop_zone_accept() -> impl Strategy<Value = Vec<String>> {
    prop_oneof![
        Just(Vec::new()),
        Just(vec!["image/*".to_string()]),
        Just(vec!["image/png".to_string()]),
        Just(vec!["text/plain".to_string()]),
    ]
}

fn arb_drop_zone_props() -> impl Strategy<Value = drop_zone::Props> {
    (
        arb_drop_zone_accept(),
        prop::option::of(0usize..=4),
        prop::option::of(0u64..=2_048),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(accept, max_files, max_file_size, disabled, read_only, invalid)| drop_zone::Props {
                id: "drop-zone".to_string(),
                accept,
                max_files,
                max_file_size,
                disabled,
                label: "Upload files".to_string(),
                allowed_operations: vec![ars_interactions::DropOperation::Move],
                name: Some("files".to_string()),
                required: false,
                invalid,
                read_only,
                activate_delay: Duration::from_millis(500),
                reset_delay: Duration::from_millis(1_500),
                get_drop_operation: None,
                on_drop: None,
                on_drop_rejected: None,
                on_drop_enter: None,
                on_drop_exit: None,
                on_drop_move: None,
                on_hover_start: None,
                on_drop_activate: None,
                on_hover_end: None,
            },
        )
}

fn arb_drop_zone_item() -> impl Strategy<Value = DragItem> {
    prop_oneof![
        "[a-z]{0,16}".prop_map(DragItem::Text),
        ("[a-z]{1,8}", 0u64..=4_096).prop_map(|(name, size)| DragItem::File {
            name: format!("{name}.png"),
            mime_type: "image/png".to_string(),
            size,
            handle: FileHandle::opaque(),
        }),
        ("[a-z]{1,8}", 0u64..=4_096).prop_map(|(name, size)| DragItem::File {
            name: format!("{name}.txt"),
            mime_type: "text/plain".to_string(),
            size,
            handle: FileHandle::opaque(),
        }),
    ]
}

fn arb_drop_zone_data() -> impl Strategy<Value = drop_zone::DragData> {
    (
        prop::collection::vec(arb_drop_zone_item(), 0..=4),
        prop::collection::vec(
            prop_oneof![
                Just("image/png".to_string()),
                Just("image/jpg".to_string()),
                Just("text/plain".to_string()),
                Just("application/json".to_string()),
            ],
            0..=4,
        ),
    )
        .prop_map(|(items, types)| drop_zone::DragData { items, types })
}

fn arb_drop_zone_event() -> impl Strategy<Value = drop_zone::Event> {
    prop_oneof![
        arb_drop_zone_data().prop_map(drop_zone::Event::DragEnter),
        arb_drop_zone_data().prop_map(drop_zone::Event::DragOver),
        Just(drop_zone::Event::DragLeave),
        arb_drop_zone_data().prop_map(drop_zone::Event::Drop),
        Just(drop_zone::Event::Reset),
        Just(drop_zone::Event::AutoReset),
        Just(drop_zone::Event::SetProps),
        Just(drop_zone::Event::DropActivate),
        any::<bool>().prop_map(|is_keyboard| drop_zone::Event::Focus { is_keyboard }),
        Just(drop_zone::Event::Blur),
    ]
}

fn arb_toggle_group_key() -> impl Strategy<Value = Key> {
    prop_oneof![
        Just(Key::str("bold")),
        Just(Key::str("italic")),
        Just(Key::str("strike")),
        Just(Key::str("code")),
    ]
}

fn arb_toggle_group_key_set() -> impl Strategy<Value = BTreeSet<Key>> {
    prop::collection::btree_set(arb_toggle_group_key(), 0..=4)
}

fn arb_toggle_group_mode() -> impl Strategy<Value = toggle_group::SelectionMode> {
    prop_oneof![
        Just(toggle_group::SelectionMode::None),
        Just(toggle_group::SelectionMode::Single),
        Just(toggle_group::SelectionMode::Multiple),
    ]
}

fn arb_toggle_group_props() -> impl Strategy<Value = toggle_group::Props> {
    (
        prop::option::of(arb_toggle_group_key_set()),
        arb_toggle_group_key_set(),
        arb_toggle_group_mode(),
        any::<bool>(),
        prop_oneof![Just(Orientation::Horizontal), Just(Orientation::Vertical)],
        prop_oneof![
            Just(ars_core::Direction::Ltr),
            Just(ars_core::Direction::Rtl)
        ],
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        arb_toggle_group_key_set(),
    )
        .prop_map(
            |(
                value,
                default_value,
                selection_mode,
                disabled,
                orientation,
                dir,
                loop_focus,
                roving_focus,
                disallow_empty_selection,
                read_only,
                disabled_items,
            )| toggle_group::Props {
                id: "toggle-group".to_string(),
                value,
                default_value,
                selection_mode,
                disabled,
                orientation,
                dir,
                loop_focus,
                roving_focus,
                aria_label: Some("Format".to_string()),
                aria_labelledby: None,
                disallow_empty_selection,
                name: Some("format".to_string()),
                invalid: false,
                required: false,
                form: None,
                read_only,
                disabled_items,
                on_change: None,
            },
        )
}

fn arb_toggle_group_event() -> impl Strategy<Value = toggle_group::Event> {
    prop_oneof![
        arb_toggle_group_key().prop_map(toggle_group::Event::SelectItem),
        arb_toggle_group_key().prop_map(toggle_group::Event::DeselectItem),
        arb_toggle_group_key().prop_map(toggle_group::Event::ToggleItem),
        (arb_toggle_group_key(), any::<bool>())
            .prop_map(|(item, is_keyboard)| { toggle_group::Event::Focus { item, is_keyboard } }),
        Just(toggle_group::Event::Blur),
        Just(toggle_group::Event::FocusNext),
        Just(toggle_group::Event::FocusPrev),
        Just(toggle_group::Event::FocusFirst),
        Just(toggle_group::Event::FocusLast),
        arb_toggle_group_key().prop_map(toggle_group::Event::RegisterItem),
        arb_toggle_group_key().prop_map(toggle_group::Event::UnregisterItem),
        Just(toggle_group::Event::Reset),
        prop::option::of(arb_toggle_group_key_set()).prop_map(toggle_group::Event::SetValue),
        Just(toggle_group::Event::SetProps),
    ]
}

fn arb_action_group_key() -> impl Strategy<Value = Key> {
    prop_oneof![
        Just(Key::str("copy")),
        Just(Key::str("delete")),
        Just(Key::str("archive")),
        Just(Key::str("share")),
    ]
}

fn arb_action_group_key_set() -> impl Strategy<Value = BTreeSet<Key>> {
    prop::collection::btree_set(arb_action_group_key(), 0..=4)
}

fn arb_action_group_selection_mode() -> impl Strategy<Value = selection::Mode> {
    prop_oneof![
        Just(selection::Mode::None),
        Just(selection::Mode::Single),
        Just(selection::Mode::Multiple),
    ]
}

fn arb_action_group_overflow_mode() -> impl Strategy<Value = action_group::OverflowMode> {
    prop_oneof![
        Just(action_group::OverflowMode::Wrap),
        Just(action_group::OverflowMode::Collapse),
        Just(action_group::OverflowMode::Menu),
    ]
}

fn arb_action_group_label_behavior() -> impl Strategy<Value = action_group::ButtonLabelBehavior> {
    prop_oneof![
        Just(action_group::ButtonLabelBehavior::Show),
        Just(action_group::ButtonLabelBehavior::Collapse),
        Just(action_group::ButtonLabelBehavior::Hide),
    ]
}

fn arb_action_group_variant() -> impl Strategy<Value = action_group::Variant> {
    prop_oneof![
        Just(action_group::Variant::Toolbar),
        Just(action_group::Variant::Outlined),
        Just(action_group::Variant::Flat),
    ]
}

fn arb_action_group_props() -> impl Strategy<Value = action_group::Props> {
    (
        prop_oneof![Just(Orientation::Horizontal), Just(Orientation::Vertical)],
        prop_oneof![
            Just(ars_core::Direction::Ltr),
            Just(ars_core::Direction::Rtl)
        ],
        arb_action_group_overflow_mode(),
        arb_action_group_variant(),
        any::<bool>(),
        arb_action_group_key_set(),
        arb_action_group_selection_mode(),
        prop::option::of(0_usize..=4),
        arb_action_group_label_behavior(),
        prop::option::of("[a-z]{1,10}".prop_map(String::from)),
        any::<bool>(),
    )
        .prop_map(
            |(
                orientation,
                dir,
                overflow_mode,
                variant,
                disabled,
                disabled_items,
                selection_mode,
                max_visible_actions,
                button_label_behavior,
                density,
                justified,
            )| action_group::Props {
                id: "action-group".to_string(),
                orientation,
                dir,
                overflow_mode,
                variant,
                disabled,
                disabled_items,
                selection_mode,
                max_visible_actions,
                button_label_behavior,
                density,
                justified,
                aria_label: Some("Actions".to_string()),
                aria_labelledby: None,
                on_action: None,
                on_selection_change: None,
            },
        )
}

fn arb_action_group_event() -> impl Strategy<Value = action_group::Event> {
    prop_oneof![
        arb_action_group_key().prop_map(action_group::Event::FocusItem),
        Just(action_group::Event::Blur),
        Just(action_group::Event::FocusNext),
        Just(action_group::Event::FocusPrev),
        Just(action_group::Event::FocusFirst),
        Just(action_group::Event::FocusLast),
        arb_action_group_key().prop_map(action_group::Event::ActivateItem),
        arb_action_group_key().prop_map(action_group::Event::SelectItem),
        (0_usize..=8).prop_map(action_group::Event::OverflowChanged),
        arb_action_group_key().prop_map(action_group::Event::RegisterItem),
        arb_action_group_key().prop_map(action_group::Event::UnregisterItem),
        Just(action_group::Event::SetProps),
    ]
}

fn arb_live_region_relevant() -> impl Strategy<Value = AriaRelevant> {
    (any::<bool>(), any::<bool>(), any::<bool>()).prop_map(|(additions, removals, text)| {
        AriaRelevant {
            additions,
            removals,
            text,
        }
    })
}

fn arb_live_region_politeness() -> impl Strategy<Value = live_region::AriaPoliteness> {
    prop_oneof![
        Just(live_region::AriaPoliteness::Off),
        Just(live_region::AriaPoliteness::Polite),
        Just(live_region::AriaPoliteness::Assertive),
    ]
}

fn arb_live_region_priority() -> impl Strategy<Value = live_region::AnnouncePriority> {
    prop_oneof![
        Just(live_region::AnnouncePriority::Normal),
        Just(live_region::AnnouncePriority::Urgent),
    ]
}

fn arb_live_region_props() -> impl Strategy<Value = live_region::Props> {
    (
        arb_live_region_politeness(),
        any::<bool>(),
        arb_live_region_relevant(),
        (0_u64..=1_000).prop_map(Duration::from_millis),
    )
        .prop_map(|(politeness, atomic, relevant, delay)| live_region::Props {
            id: "live-region".to_string(),
            politeness,
            atomic,
            relevant,
            delay,
        })
}

fn arb_live_region_event() -> impl Strategy<Value = live_region::Event> {
    prop_oneof![
        (
            "[a-zA-Z0-9 _-]{1,24}".prop_map(String::from),
            arb_live_region_priority(),
        )
            .prop_map(|(message, priority)| live_region::Event::Announce { message, priority }),
        Just(live_region::Event::Clear),
        Just(live_region::Event::Rendered),
        Just(live_region::Event::SetProps),
    ]
}

fn arb_focus_scope_props() -> impl Strategy<Value = focus_scope::Props> {
    (any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
        |(trapped, contain, auto_focus, restore_focus)| focus_scope::Props {
            id: "focus-scope".to_string(),
            trapped,
            contain,
            auto_focus,
            restore_focus,
        },
    )
}

fn arb_optional_focus_target() -> impl Strategy<Value = Option<String>> {
    prop::option::of("[a-zA-Z0-9-]{1,16}".prop_map(String::from))
}

fn arb_focus_scope_event() -> impl Strategy<Value = focus_scope::Event> {
    prop_oneof![
        (any::<bool>(), arb_optional_focus_target()).prop_map(|(trapped, saved_focus_id)| {
            focus_scope::Event::Activate {
                trapped,
                saved_focus_id,
            }
        }),
        any::<bool>().prop_map(|restore_focus| focus_scope::Event::Deactivate { restore_focus }),
        Just(focus_scope::Event::TrapFocus),
        Just(focus_scope::Event::ReleaseTrap),
        Just(focus_scope::Event::RestoreFocus),
        Just(focus_scope::Event::FocusFirst),
        Just(focus_scope::Event::FocusLast),
    ]
}

fn arb_swap_props() -> impl Strategy<Value = swap::Props> {
    (
        prop::option::of(any::<bool>()),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(checked, default_checked, disabled)| swap::Props {
            id: "swap".to_string(),
            checked,
            default_checked,
            disabled,
            label: None,
            animation: swap::Animation::None,
            on_change: None,
        })
}

fn arb_swap_event() -> impl Strategy<Value = swap::Event> {
    prop_oneof![
        Just(swap::Event::Toggle),
        Just(swap::Event::SetOn),
        Just(swap::Event::SetOff),
        any::<bool>().prop_map(swap::Event::SetDisabled),
        prop::option::of(any::<bool>()).prop_map(swap::Event::SetValue),
        any::<bool>().prop_map(|is_keyboard| swap::Event::Focus { is_keyboard }),
        Just(swap::Event::Blur),
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
    #![proptest_config(super::common::proptest_config())]

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
    fn proptest_toggle_event_sequences_preserve_invariants(
        props in arb_toggle_props(),
        events in prop::collection::vec(arb_toggle_event(), 0..128),
    ) {
        let mut service = Service::<toggle::Machine>::new(
            props,
            &Env::default(),
            &toggle::Messages,
        );

        for event in events {
            let before_pressed = *service.context().pressed.get();

            let before_disabled = service.context().disabled;

            let value_event = matches!(
                event,
                toggle::Event::Toggle | toggle::Event::TurnOn | toggle::Event::TurnOff
            );

            drop(service.send(event));

            let state = service.state();
            let ctx = service.context();

            prop_assert_eq!(matches!(state, toggle::State::On), *ctx.pressed.get());
            prop_assert!(
                !ctx.focus_visible || ctx.focused,
                "focus-visible cannot outlive focus"
            );

            if before_disabled && value_event {
                prop_assert_eq!(
                    *ctx.pressed.get(),
                    before_pressed,
                    "disabled toggle must not change pressed value"
                );
            }
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_toggle_group_event_sequences_preserve_invariants(
        props in arb_toggle_group_props(),
        events in prop::collection::vec(arb_toggle_group_event(), 0..128),
    ) {
        let mut service = Service::<toggle_group::Machine>::new(
            props,
            &Env::default(),
            &toggle_group::Messages::default(),
        );

        for event in events {
            let before_value = service.context().value.get().clone();
            let before_disabled = service.context().disabled;
            let before_read_only = service.props().read_only;

            let value_item_event = matches!(
                event,
                toggle_group::Event::SelectItem(_)
                    | toggle_group::Event::DeselectItem(_)
                    | toggle_group::Event::ToggleItem(_)
            );

            drop(service.send(event));

            let state = service.state();
            let ctx = service.context();

            match state {
                toggle_group::State::Idle => {
                    prop_assert!(ctx.focused_item.is_none());
                    prop_assert!(!ctx.focus_visible);
                }

                toggle_group::State::Focused { item } => {
                    prop_assert_eq!(ctx.focused_item.as_ref(), Some(item));
                    prop_assert!(
                        ctx.registered_items.iter().any(|registered| registered == item),
                        "focused item must be registered"
                    );
                    prop_assert!(
                        !ctx.disabled_items.contains(item),
                        "focused item must not be item-disabled"
                    );
                }
            }

            match ctx.selection_mode {
                toggle_group::SelectionMode::None => {
                    prop_assert!(ctx.value.get().is_empty(), "none mode cannot select");
                }

                toggle_group::SelectionMode::Single => {
                    prop_assert!(ctx.value.get().len() <= 1, "single mode selects at most one");
                }

                toggle_group::SelectionMode::Multiple => {}
            }

            if before_disabled && value_item_event {
                prop_assert_eq!(
                    ctx.value.get(),
                    &before_value,
                    "disabled group cannot change selection from item value events"
                );
            }

            if before_read_only && value_item_event {
                prop_assert_eq!(
                    ctx.value.get(),
                    &before_value,
                    "read-only group cannot change selection from item events"
                );
            }

            let registered = ctx.registered_items.iter().collect::<BTreeSet<_>>();

            prop_assert_eq!(
                registered.len(),
                ctx.registered_items.len(),
                "registered item list must be deduplicated"
            );

            if let Some(focused) = &ctx.focused_item {
                prop_assert!(ctx.registered_items.iter().any(|item| item == focused));
                prop_assert!(!ctx.disabled_items.contains(focused));
            }

            if ctx.roving_focus {
                let enabled = ctx
                    .registered_items
                    .iter()
                    .filter(|item| !ctx.disabled_items.contains(*item))
                    .collect::<Vec<_>>();

                if !enabled.is_empty() {
                    let api = service.connect(&|_| {});

                    let zero_count = enabled
                        .iter()
                        .filter(|item| {
                            api.item_attrs(item)
                                .get(&HtmlAttr::TabIndex)
                                .is_some_and(|value| value == "0")
                        })
                        .count();

                    prop_assert_eq!(
                        zero_count,
                        1,
                        "exactly one enabled item anchors roving tabindex"
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_action_group_event_sequences_preserve_invariants(
        props in arb_action_group_props(),
        events in prop::collection::vec(arb_action_group_event(), 0..128),
    ) {
        let mut service = Service::<action_group::Machine>::new(
            props,
            &Env::default(),
            &action_group::Messages::default(),
        );

        for event in events {
            let before_selected = service.context().selected_items.clone();
            let before_disabled = service.context().disabled;

            let value_event = matches!(
                event,
                action_group::Event::ActivateItem(_) | action_group::Event::SelectItem(_)
            );

            drop(service.send(event));

            let state = service.state();
            let ctx = service.context();

            match state {
                action_group::State::Idle => {
                    prop_assert!(ctx.focused_item.is_none());
                }

                action_group::State::Focused { item } => {
                    prop_assert_eq!(ctx.focused_item.as_ref(), Some(item));
                    prop_assert!(
                        ctx.registered_items.iter().any(|registered| registered == item),
                        "focused item must be registered"
                    );
                    prop_assert!(
                        !service.props().disabled_items.contains(item),
                        "focused item must not be item-disabled"
                    );
                }
            }

            match service.props().selection_mode {
                selection::Mode::None => {
                    prop_assert!(ctx.selected_items.is_empty(), "none mode cannot select");
                }

                selection::Mode::Single => {
                    prop_assert!(
                        ctx.selected_items.len() <= 1,
                        "single mode selects at most one"
                    );
                }

                selection::Mode::Multiple => {}
            }

            if ctx.overflow_count <= ctx.registered_items.len() {
                prop_assert_eq!(
                    ctx.visible_count + ctx.overflow_count,
                    ctx.registered_items.len(),
                    "visible plus overflowed items should cover registered items"
                );
            } else {
                prop_assert_eq!(
                    ctx.visible_count,
                    0,
                    "overflow beyond the registered count saturates visible count at zero"
                );
            }

            let registered = ctx.registered_items.iter().collect::<BTreeSet<_>>();

            prop_assert_eq!(
                registered.len(),
                ctx.registered_items.len(),
                "registered item list must be deduplicated"
            );

            if before_disabled && value_event {
                prop_assert_eq!(
                    &ctx.selected_items,
                    &before_selected,
                    "disabled action group cannot change selection from value events"
                );
            }
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_live_region_event_sequences_preserve_invariants(
        props in arb_live_region_props(),
        events in prop::collection::vec(arb_live_region_event(), 0..128),
    ) {
        let mut service = Service::<live_region::Machine>::new(
            props,
            &Env::default(),
            &live_region::Messages,
        );

        for event in events {
            let was_clear = matches!(event, live_region::Event::Clear);

            let was_rendered = matches!(event, live_region::Event::Rendered);

            let was_announcing = matches!(service.state(), live_region::State::Announcing);

            let queued_has_urgent = service
                .context()
                .queue
                .iter()
                .any(|queued| queued.priority == live_region::AnnouncePriority::Urgent);

            let queued_has_normal = service
                .context()
                .queue
                .iter()
                .any(|queued| queued.priority == live_region::AnnouncePriority::Normal);

            drop(service.send(event));

            let state = service.state();
            let ctx = service.context();

            prop_assert!(
                ctx.pending_message.is_none() || matches!(state, live_region::State::Announcing),
                "pending message requires Announcing state"
            );
            prop_assert!(
                ctx.messages.len() <= 1,
                "only one rendered announcement may be present"
            );
            prop_assert!(
                ctx.queue
                    .windows(2)
                    .all(|window| window[0].sequence < window[1].sequence),
                "queue sequence must preserve insertion order"
            );

            if was_clear {
                prop_assert_eq!(state, &live_region::State::Idle);
                prop_assert!(ctx.messages.is_empty(), "Clear empties rendered messages");
                prop_assert!(ctx.queue.is_empty(), "Clear empties queued messages");
                prop_assert_eq!(&ctx.pending_message, &None, "Clear drops pending message");
            }

            if was_rendered && was_announcing && queued_has_urgent && queued_has_normal {
                prop_assert_eq!(
                    ctx.current_priority,
                    live_region::AnnouncePriority::Urgent,
                    "urgent queued messages are selected before normal messages"
                );
            }
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_swap_event_sequences_preserve_invariants(
        props in arb_swap_props(),
        events in prop::collection::vec(arb_swap_event(), 0..128),
    ) {
        let mut service = Service::<swap::Machine>::new(
            props,
            &Env::default(),
            &swap::Messages::default(),
        );

        for event in events {
            let before_checked = *service.context().checked.get();

            let before_disabled = service.context().disabled;

            let value_event = matches!(
                event,
                swap::Event::Toggle | swap::Event::SetOn | swap::Event::SetOff
            );

            drop(service.send(event));

            let state = service.state();
            let ctx = service.context();

            prop_assert_eq!(matches!(state, swap::State::On), *ctx.checked.get());

            if before_disabled && value_event {
                prop_assert_eq!(
                    *ctx.checked.get(),
                    before_checked,
                    "disabled swap must not change checked value"
                );
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

fn arb_optional_short_filename() -> impl Strategy<Value = Option<String>> {
    prop_oneof![Just(None), "[a-z]{1,12}\\.[a-z]{2,4}".prop_map(Some),]
}

fn arb_optional_mime_type() -> impl Strategy<Value = Option<String>> {
    prop_oneof![Just(None), "[a-z]{1,12}/[a-z]{1,12}".prop_map(Some),]
}

fn arb_download_trigger_props() -> impl Strategy<Value = download_trigger::Props> {
    (
        arb_short_id(),
        prop_oneof![
            "[a-z]{1,16}".prop_map(|segment| format!("/{segment}")),
            Just(String::from("https://example.com/file")),
        ],
        arb_optional_short_filename(),
        arb_optional_mime_type(),
        any::<bool>(),
        prop_oneof![Just(None), Just(Some(String::from("https://example.com"))),],
    )
        .prop_map(
            |(id, href, filename, mime_type, disabled, document_origin)| download_trigger::Props {
                id,
                href,
                filename,
                mime_type,
                disabled,
                document_origin,
            },
        )
}

fn arb_download_trigger_cross_origin_props() -> impl Strategy<Value = download_trigger::Props> {
    (arb_short_id(), arb_optional_short_filename(), any::<bool>()).prop_map(
        |(id, filename, disabled)| download_trigger::Props {
            id,
            href: String::from("https://cdn.example/asset.bin"),
            filename,
            mime_type: None,
            disabled,
            document_origin: Some(String::from("https://app.example")),
        },
    )
}

proptest! {
    #![proptest_config(super::common::proptest_config())]

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

    // ── DownloadTrigger ────────────────────────────────────────────

    /// `Api::part_attrs(Part::Root)` always equals `Api::root_attrs()`.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_download_trigger_part_root_dispatch_equals_root_attrs(
        props in arb_download_trigger_props(),
    ) {
        let api = download_trigger::Api::new(
            props,
            ars_i18n::locales::en_us(),
            download_trigger::Messages::default(),
        );

        prop_assert_eq!(
            api.part_attrs(download_trigger::Part::Root),
            api.root_attrs()
        );
    }

    /// Root attrs always carry canonical scope/part data attributes.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_download_trigger_root_attrs_always_have_scope_and_part(
        props in arb_download_trigger_props(),
    ) {
        let attrs = download_trigger::Api::new(
            props,
            ars_i18n::locales::en_us(),
            download_trigger::Messages::default(),
        )
        .root_attrs();

        prop_assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("download-trigger")
        );
        prop_assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
    }

    /// Relative hrefs remain native-download eligible without `document_origin`.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_download_trigger_relative_href_is_always_native_eligible(
        props in arb_download_trigger_props()
            .prop_filter("relative href", |p| {
                p.href.starts_with('/') && !p.href.starts_with("//")
            }),
    ) {
        let api = download_trigger::Api::new(
            props,
            ars_i18n::locales::en_us(),
            download_trigger::Messages::default(),
        );

        prop_assert!(api.native_download_eligible());
        prop_assert!(!api.needs_blob_fallback());
        prop_assert!(api.root_attrs().contains(&HtmlAttr::Download));
    }

    /// Cross-origin HTTP(S) emits the fallback hook instead of `download`.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_download_trigger_cross_origin_signals_blob_fallback(
        props in arb_download_trigger_cross_origin_props(),
    ) {
        let api = download_trigger::Api::new(
            props,
            ars_i18n::locales::en_us(),
            download_trigger::Messages::default(),
        );

        prop_assert!(!api.native_download_eligible());
        prop_assert!(api.needs_blob_fallback());

        let attrs = api.root_attrs();

        prop_assert_eq!(attrs.get(&HtmlAttr::Download), None);
        prop_assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-download-fallback")),
            Some(download_trigger::DOWNLOAD_FALLBACK_REQUIRED)
        );
    }

    /// `Api::props()` round-trip for DownloadTrigger.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_download_trigger_api_props_round_trip(props in arb_download_trigger_props()) {
        let original = props.clone();

        let api = download_trigger::Api::new(
            props,
            ars_i18n::locales::en_us(),
            download_trigger::Messages::default(),
        );

        prop_assert_eq!(api.props(), &original);
        prop_assert_eq!(api.id(), original.id.as_str());
        prop_assert_eq!(api.is_disabled(), original.disabled);
        prop_assert_eq!(api.filename(), original.filename.as_deref());
    }

    // ── ErrorBoundary ─────────────────────────────────────────────
    //
    // ErrorBoundary's framework-agnostic core is an attribute-only
    // surface driven by a single input: the captured error count. The
    // proptests below pin the invariants the adapter wrappers depend on
    // — the count round-trips through `error_count()`, the count-as-
    // string survives the `data-ars-error-count` attribute, every
    // anatomy part emits the canonical scope/part pair, and the alert
    // markup is invariant under count changes (count is the only field
    // that affects Root's payload).

    /// `Api::error_count()` round-trips any non-negative count we feed it.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_error_boundary_count_round_trips(count in 0usize..=10_000) {
        let api = error_boundary::Api::new(count);

        prop_assert_eq!(api.error_count(), count);
    }

    /// `data-ars-error-count` is the `Display` of the count, for any count.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_error_boundary_count_attr_matches_count(count in 0usize..=10_000) {
        let attrs = error_boundary::Api::new(count).root_attrs();

        let expected = count.to_string();

        prop_assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-error-count")),
            Some(expected.as_str())
        );
    }

    /// Every anatomy part emits the canonical scope and matching part
    /// data attribute, regardless of error count. The connect-API
    /// dispatch must produce the same `AttrMap` as the inherent helper
    /// for each part.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_error_boundary_part_dispatch_is_canonical(count in 0usize..=10_000) {
        let api = error_boundary::Api::new(count);

        let cases = [
            (error_boundary::Part::Root,    "root",    api.root_attrs()),
            (error_boundary::Part::Message, "message", api.message_attrs()),
            (error_boundary::Part::List,    "list",    api.list_attrs()),
            (error_boundary::Part::Item,    "item",    api.item_attrs()),
        ];

        for (part, name, helper_attrs) in cases {
            let dispatched = api.part_attrs(part);

            prop_assert_eq!(
                dispatched.get(&HtmlAttr::Data("ars-scope")),
                Some("error-boundary"),
                "scope missing for part {}", name
            );
            prop_assert_eq!(
                dispatched.get(&HtmlAttr::Data("ars-part")),
                Some(name),
            );
            prop_assert_eq!(
                dispatched, helper_attrs,
                "ConnectApi dispatch must equal inherent helper for part {}", name
            );
        }
    }

    /// The accessibility primitives (`role="alert"`, `aria-live`,
    /// `aria-atomic`) are constant across all error counts. The count
    /// is the only input that influences `data-ars-error-count`; every
    /// other Root attr stays put.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_error_boundary_aria_primitives_are_count_invariant(
        a in 0usize..=10_000,
        b in 0usize..=10_000,
    ) {
        let attrs_a = error_boundary::Api::new(a).root_attrs();
        let attrs_b = error_boundary::Api::new(b).root_attrs();

        for attr in [
            HtmlAttr::Role,
            HtmlAttr::Aria(ars_core::AriaAttr::Live),
            HtmlAttr::Aria(ars_core::AriaAttr::Atomic),
            HtmlAttr::Data("ars-scope"),
            HtmlAttr::Data("ars-part"),
            HtmlAttr::Data("ars-error"),
        ] {
            prop_assert_eq!(
                attrs_a.get(&attr),
                attrs_b.get(&attr),
                "attr {:?} should be count-invariant", attr
            );
        }
    }
}

// ── Highlight (utility::highlight, gated on the `i18n` feature) ────

#[cfg(feature = "i18n")]
fn arb_match_strategy() -> impl Strategy<Value = highlight::MatchStrategy> {
    prop_oneof![
        Just(highlight::MatchStrategy::Contains),
        Just(highlight::MatchStrategy::StartsWith),
        Just(highlight::MatchStrategy::Fuzzy),
    ]
}

#[cfg(feature = "i18n")]
fn arb_highlight_text() -> impl Strategy<Value = String> {
    // Mix of ASCII, multi-byte UTF-8 (ß, İ, é), and whitespace to
    // exercise byte-boundary and case-folding paths together.
    "[a-zA-Z0-9 _\u{00DF}\u{0130}\u{00E9}]{0,32}".prop_map(String::from)
}

#[cfg(feature = "i18n")]
fn arb_highlight_query() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(
        "[a-zA-Z0-9 \u{00DF}\u{0130}]{0,8}".prop_map(String::from),
        0..4,
    )
}

#[cfg(feature = "i18n")]
fn arb_highlight_props() -> impl Strategy<Value = highlight::Props> {
    (
        arb_highlight_query(),
        arb_highlight_text(),
        any::<bool>(),
        arb_match_strategy(),
    )
        .prop_map(|(query, text, ignore_case, match_strategy)| {
            highlight::Props::new()
                .query(query)
                .text(text)
                .ignore_case(ignore_case)
                .match_strategy(match_strategy)
        })
}

#[cfg(feature = "i18n")]
fn arb_highlight_locale() -> impl Strategy<Value = Locale> {
    // Mix of locale families that exercise different case-folding paths:
    // - en-US: default Unicode fold.
    // - tr / az: Turkic dotted/dotless-I fold (CaseMapper::fold_turkic_string).
    // - de: German eszett expansion `ß → ss`.
    // - el: Greek final-sigma collapse `Σ/σ/ς → σ`.
    // - lt: Lithuanian combining-dot handling.
    // - hy: Armenian — exercises a script with case but no special tailoring.
    // - ar: a script with no case at all (the fold is the identity).
    prop_oneof![
        Just(Locale::parse("en-US").expect("en-US must parse")),
        Just(Locale::parse("tr").expect("tr must parse")),
        Just(Locale::parse("az").expect("az must parse")),
        Just(Locale::parse("de").expect("de must parse")),
        Just(Locale::parse("el").expect("el must parse")),
        Just(Locale::parse("lt").expect("lt must parse")),
        Just(Locale::parse("hy").expect("hy must parse")),
        Just(Locale::parse("ar").expect("ar must parse")),
    ]
}

#[cfg(feature = "i18n")]
proptest! {
    #![proptest_config(super::common::proptest_config())]

    /// For any combination of props and locale, `highlight_chunks` must:
    ///
    /// 1. **Never panic** (covered by the test reaching its end).
    /// 2. **Roundtrip the text** — concatenating all chunk slices in order
    ///    yields the original `props.text` byte-for-byte. Catches any
    ///    range / index / byte-map regression silently dropping or
    ///    duplicating text.
    /// 3. **Never emit two adjacent highlighted chunks** — the agnostic
    ///    core's adjacency-merge contract (spec §3.1) must hold for every
    ///    strategy and every query combination.
    /// 4. **Never emit empty chunks** — zero-length segments are skipped
    ///    by `build_chunks`; this guards against silent regressions there.
    /// 5. **Empty query (or all-empty queries)** must produce exactly one
    ///    non-highlighted chunk wrapping the full text, when the text is
    ///    non-empty. Empty text yields an empty `Vec`.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_highlight_chunks_invariants(
        props in arb_highlight_props(),
        locale in arb_highlight_locale(),
    ) {
        let chunks = highlight::highlight_chunks(&props, &locale);

        // (2) roundtrip
        let concatenated: String = chunks.iter().map(|c| c.text).collect();
        prop_assert_eq!(
            concatenated.as_str(),
            props.text.as_str(),
            "chunks must reconstruct the original text"
        );

        // (3) no two adjacent highlighted chunks
        for window in chunks.windows(2) {
            prop_assert!(
                !(window[0].highlighted && window[1].highlighted),
                "adjacency-merge regression: {:?}", window
            );
        }

        // (4) no empty chunks
        for chunk in &chunks {
            prop_assert!(
                !chunk.text.is_empty(),
                "empty chunk emitted: {:?}", chunk
            );
        }

        // (5) empty-query / empty-text special cases
        let all_queries_empty = props.query.iter().all(String::is_empty);
        if props.text.is_empty() {
            prop_assert!(chunks.is_empty(), "empty text should yield empty Vec");
        } else if props.query.is_empty() || all_queries_empty {
            prop_assert_eq!(chunks.len(), 1, "empty query → exactly one chunk");
            prop_assert!(!chunks[0].highlighted, "empty-query chunk must not be highlighted");
            prop_assert_eq!(chunks[0].text, props.text.as_str());
        }
    }
}

proptest! {
    #![proptest_config(super::common::proptest_config())]

    /// Arbitrary DropZone event sequences must keep the public state and
    /// connect API internally consistent: drag-over state owns the drop-target
    /// marker, named enabled instances expose stored accepted form data, and
    /// disabled/read-only instances never enter drag/drop states.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_drop_zone_event_sequences_preserve_invariants(
        props in arb_drop_zone_props(),
        events in prop::collection::vec(arb_drop_zone_event(), 0..128),
    ) {
        let initially_disabled_or_readonly = props.disabled || props.read_only;

        let mut service = Service::<drop_zone::Machine>::new(
            props,
            &Env::default(),
            &drop_zone::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            let attrs = service.connect(&|_| {}).root_attrs();

            let drag_over_attr = attrs.get(&HtmlAttr::Data("ars-drag-over"));

            prop_assert_eq!(
                matches!(service.state(), drop_zone::State::DragOver),
                service.context().is_drop_target,
                "DragOver state and is_drop_target must agree"
            );
            prop_assert_eq!(
                matches!(service.state(), drop_zone::State::DragOver),
                drag_over_attr == Some("true"),
                "root data-ars-drag-over must track DragOver state"
            );

            if service.state() != &drop_zone::State::DragOver {
                prop_assert!(
                    !service.context().valid_drag,
                    "valid_drag must clear outside DragOver"
                );
            }

            if service.props().name.is_none() || service.context().disabled {
                let form_data = service.connect(&|_| {}).form_data().to_vec();
                prop_assert!(
                    form_data.is_empty(),
                    "form_data must be empty for unnamed or disabled instances"
                );
            } else {
                let form_data = service.connect(&|_| {}).form_data().to_vec();
                prop_assert_eq!(
                    form_data.as_slice(),
                    service.context().dropped_items.as_slice(),
                    "form_data must expose stored accepted items for named enabled instances"
                );
            }

            if initially_disabled_or_readonly {
                prop_assert!(
                    !matches!(
                        service.state(),
                        drop_zone::State::DragOver
                            | drop_zone::State::DropAccepted
                            | drop_zone::State::DropRejected
                    ),
                    "disabled/read-only DropZone must ignore drag/drop transitions"
                );
            }
        }
    }

    /// `Activate → Deactivate { restore_focus: false }` always lands at
    /// `State::Inactive` with `saved_focus = None`, regardless of any
    /// intermediate events. Intermediate events may toggle the scope in
    /// and out of `Active`, but the final forced `Deactivate(false)`
    /// from `Active` clears the saved focus via its apply step.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_focus_scope_activate_deactivate_round_trip(
        props in arb_focus_scope_props(),
        intermediate in prop::collection::vec(arb_focus_scope_event(), 0..64),
        saved_focus_id in arb_optional_focus_target(),
    ) {
        let mut service = Service::<focus_scope::Machine>::new(
            props,
            &Env::default(),
            &focus_scope::Messages,
        );

        // Force-activate so the round-trip always exercises both halves.
        drop(service.send(focus_scope::Event::Activate {
            trapped: true,
            saved_focus_id,
        }));

        for event in intermediate {
            drop(service.send(event));
        }

        // Re-activate if the intermediate sequence landed us back at
        // `Inactive`. Without this the final `Deactivate(false)` would be
        // a no-op (the wildcard arm ignores it) and any leftover
        // `saved_focus` from an earlier `Deactivate(true)` would survive.
        if matches!(service.state(), focus_scope::State::Inactive) {
            drop(service.send(focus_scope::Event::Activate {
                trapped: false,
                saved_focus_id: Some("force-active".to_string()),
            }));
        }

        // Force back to Inactive without restoration. Any active scope
        // MUST return to Inactive after a Deactivate(false); the apply
        // step clears `saved_focus` to drop the stale token.
        drop(service.send(focus_scope::Event::Deactivate { restore_focus: false }));

        prop_assert_eq!(service.state(), &focus_scope::State::Inactive);
        prop_assert!(service.context().saved_focus.is_none());
    }

    /// `TrapFocus` and `ReleaseTrap` only have a state-affecting effect
    /// while the scope is `Active`; they are ignored from `Inactive`.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_focus_scope_trap_release_only_changes_state_in_active(
        props in arb_focus_scope_props(),
        events in prop::collection::vec(arb_focus_scope_event(), 0..32),
    ) {
        let mut service = Service::<focus_scope::Machine>::new(
            props,
            &Env::default(),
            &focus_scope::Messages,
        );

        for event in events {
            let was_active = matches!(service.state(), focus_scope::State::Active { .. });

            let result = service.send(event.clone());

            match event {
                focus_scope::Event::TrapFocus | focus_scope::Event::ReleaseTrap
                    if !was_active =>
                {
                    prop_assert!(
                        !result.state_changed,
                        "{:?} from Inactive must be ignored",
                        event,
                    );
                }
                _ => {}
            }

            // The trapped flag and the State::Active variant always agree.
            match service.state() {
                focus_scope::State::Inactive => {
                    // No further invariant — saved_focus may be set or cleared.
                }

                focus_scope::State::Active { trapped } => {
                    // Empty arm: we just assert the variant carries the
                    // current `trapped` flag, which is structural.
                    let _ = trapped;
                }
            }
        }
    }

    /// Focus-navigation events (`FocusFirst`, `FocusLast`, `RestoreFocus`)
    /// never change the high-level state — they either emit an effect
    /// intent (when their state precondition holds) or are no-ops.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_focus_scope_navigation_events_never_leave_state(
        props in arb_focus_scope_props(),
        events in prop::collection::vec(arb_focus_scope_event(), 0..32),
    ) {
        let mut service = Service::<focus_scope::Machine>::new(
            props,
            &Env::default(),
            &focus_scope::Messages,
        );

        for event in events {
            let before = *service.state();

            let result = service.send(event.clone());

            if matches!(
                event,
                focus_scope::Event::FocusFirst
                    | focus_scope::Event::FocusLast
                    | focus_scope::Event::RestoreFocus,
            ) {
                prop_assert!(
                    !result.state_changed,
                    "{:?} must not change the high-level state (it only emits an effect intent)",
                    event,
                );
                prop_assert_eq!(service.state(), &before);
            }
        }
    }
}
